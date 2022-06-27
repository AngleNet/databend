// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use async_channel::Sender;
use common_arrow::arrow_format::flight::data::FlightData;
use common_arrow::arrow_format::flight::service::flight_service_client::FlightServiceClient;
use common_base::base::tokio::task::JoinHandle;
use common_base::base::Runtime;
use common_base::base::Thread;
use common_base::base::TrySpawn;
use common_base::infallible::Mutex;
use common_base::infallible::ReentrantMutex;
use common_datavalues::DataSchemaRef;
use common_exception::ErrorCode;
use common_exception::Result;
use common_grpc::ConnectionFactory;
use common_planners::PlanNode;
use futures::StreamExt;
use tonic::Streaming;

use crate::api::rpc::exchange::exchange_channel::FragmentReceiver;
use crate::api::rpc::exchange::exchange_channel::FragmentSender;
use crate::api::rpc::exchange::exchange_channel_receiver::ExchangeReceiver;
use crate::api::rpc::exchange::exchange_params::ExchangeParams;
use crate::api::rpc::exchange::exchange_params::MergeExchangeParams;
use crate::api::rpc::exchange::exchange_params::ShuffleExchangeParams;
use crate::api::rpc::exchange::exchange_sink::ExchangeSink;
use crate::api::rpc::exchange::exchange_source::ExchangeSource;
use crate::api::rpc::flight_actions::PreparePipeline;
use crate::api::rpc::flight_actions::PreparePublisher;
use crate::api::rpc::flight_scatter_hash::HashFlightScatter;
use crate::api::rpc::packet::DataPacket;
use crate::api::rpc::packet::ExecutePacket;
use crate::api::DataExchange;
use crate::api::ExecutorPacket;
use crate::api::FlightAction;
use crate::api::FlightClient;
use crate::api::FragmentPacket;
use crate::api::PrepareChannel;
use crate::api::rpc::exchange::exchange_channel_sender::ExchangeSender;
use crate::interpreters::QueryFragmentsActions;
use crate::pipelines::new::executor::PipelineCompleteExecutor;
use crate::pipelines::new::NewPipe;
use crate::pipelines::new::NewPipeline;
use crate::pipelines::new::QueryPipelineBuilder;
use crate::sessions::QueryContext;
use crate::Config;

pub struct DataExchangeManager {
    config: Config,
    queries_coordinator: ReentrantMutex<HashMap<String, QueryCoordinator>>,
}

impl DataExchangeManager {
    pub fn create(config: Config) -> Arc<DataExchangeManager> {
        Arc::new(DataExchangeManager {
            config,
            queries_coordinator: ReentrantMutex::new(HashMap::new()),
        })
    }

    // Create connections for cluster all nodes. We will push data through this connection.
    pub async fn handle_prepare_publisher(&self, packet: &PrepareChannel) -> Result<()> {
        let mut exchange_senders = HashMap::with_capacity(packet.target_2_fragments.len());
        for (target, _fragments) in &packet.target_2_fragments {
            let config = self.config.clone();
            exchange_senders.insert(
                target.clone(),
                ExchangeSender::create(config, packet, target).await?,
            );
        }

        let mut queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.get_mut(&packet.query_id) {
            None => Err(ErrorCode::LogicalError(format!(
                "Query {} not found in cluster.",
                packet.query_id
            ))),
            Some(coordinator) => coordinator.init_publisher(exchange_senders),
        }
    }

    // Execute query in background
    pub fn handle_execute_pipeline(&self, query_id: &str) -> Result<()> {
        let mut queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.get_mut(query_id) {
            None => Err(ErrorCode::LogicalError(format!(
                "Query {} not found in cluster.",
                query_id
            ))),
            Some(coordinator) => coordinator.execute_pipeline(),
        }
    }

    // Receive data by cluster other nodes.
    pub async fn handle_do_put(
        &self,
        id: &str,
        source: &str,
        stream: Streaming<FlightData>,
    ) -> Result<JoinHandle<()>> {
        let mut queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.get_mut(id) {
            None => Err(ErrorCode::LogicalError(format!(
                "Query {} not found in cluster.",
                id
            ))),
            Some(coordinator) => coordinator.receive_data(source, stream),
        }
    }

    pub fn shutdown_query(&self, query_id: &str, cause: Option<ErrorCode>) -> Result<()> {
        let mut queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.remove(query_id) {
            None => Err(ErrorCode::LogicalError(format!(
                "Query {} not found in cluster.",
                query_id
            ))),
            Some(mut coordinator) => coordinator.shutdown(cause),
        }
    }

    // Create a pipeline based on query plan
    pub fn handle_prepare_pipeline(
        &self,
        ctx: &Arc<QueryContext>,
        prepare: &PreparePipeline,
    ) -> Result<()> {
        let mut queries_coordinator = self.queries_coordinator.lock();

        let executor_packet = &prepare.executor_packet;
        // TODO: When the query is not executed for a long time after submission, we need to remove it
        match queries_coordinator.entry(executor_packet.query_id.to_owned()) {
            Entry::Occupied(_) => Err(ErrorCode::LogicalError(format!(
                "Already exists query id {:?}",
                executor_packet.query_id
            ))),
            Entry::Vacant(entry) => {
                let query_coordinator = QueryCoordinator::create(ctx, executor_packet)?;
                let query_coordinator = entry.insert(query_coordinator);
                query_coordinator.prepare_pipeline()?;

                if executor_packet.executor != executor_packet.request_executor {
                    query_coordinator.prepare_subscribes_channel(executor_packet)?;
                }

                Ok(())
            }
        }
    }

    async fn create_client(&self, address: &str) -> Result<FlightClient> {
        return match self.config.tls_query_cli_enabled() {
            true => Ok(FlightClient::new(FlightServiceClient::new(
                ConnectionFactory::create_rpc_channel(
                    address.to_owned(),
                    None,
                    Some(self.config.query.to_rpc_client_tls_config()),
                )
                    .await?,
            ))),
            false => Ok(FlightClient::new(FlightServiceClient::new(
                ConnectionFactory::create_rpc_channel(address.to_owned(), None, None).await?,
            ))),
        };
    }

    async fn prepare_pipeline(&self, packets: &[ExecutorPacket], timeout: u64) -> Result<()> {
        for executor_packet in packets {
            if !executor_packet
                .executors_info
                .contains_key(&executor_packet.executor)
            {
                return Err(ErrorCode::LogicalError(format!(
                    "Not found {} node in cluster",
                    &executor_packet.executor
                )));
            }

            let executor_packet = executor_packet.clone();
            let executor_info = &executor_packet.executors_info[&executor_packet.executor];
            let mut connection = self.create_client(&executor_info.flight_address).await?;
            let action = FlightAction::PreparePipeline(PreparePipeline { executor_packet });
            connection.execute_action(action, timeout).await?;
        }

        Ok(())
    }

    async fn prepare_channel(&self, packets: Vec<PrepareChannel>, timeout: u64) -> Result<()> {
        for publisher_packet in packets.into_iter() {
            if !publisher_packet
                .data_endpoints
                .contains_key(&publisher_packet.executor)
            {
                return Err(ErrorCode::LogicalError(format!(
                    "Not found {} node in cluster",
                    &publisher_packet.executor
                )));
            }

            let executor_info = &publisher_packet.data_endpoints[&publisher_packet.executor];
            let mut connection = self.create_client(&executor_info.flight_address).await?;
            let action = FlightAction::PreparePublisher(PreparePublisher { publisher_packet });
            connection.execute_action(action, timeout).await?;
        }

        Ok(())
    }

    async fn execute_pipeline(&self, packets: Vec<ExecutePacket>, timeout: u64) -> Result<()> {
        for execute_packet in packets.into_iter() {
            if !execute_packet
                .executors_info
                .contains_key(&execute_packet.executor)
            {
                return Err(ErrorCode::LogicalError(format!(
                    "Not found {} node in cluster",
                    &execute_packet.executor
                )));
            }

            let executor_info = &execute_packet.executors_info[&execute_packet.executor];
            let mut connection = self.create_client(&executor_info.flight_address).await?;
            let action = FlightAction::ExecutePipeline(execute_packet.query_id);
            connection.execute_action(action, timeout).await?;
        }

        Ok(())
    }

    pub async fn commit_actions(
        &self,
        ctx: Arc<QueryContext>,
        actions: QueryFragmentsActions,
    ) -> Result<NewPipeline> {
        let settings = ctx.get_settings();
        let timeout = settings.get_flight_client_timeout()?;
        let root_actions = actions.get_root_actions()?;
        let root_fragment_id = root_actions.fragment_id.to_owned();

        // Submit distributed tasks to all nodes.
        let prepare_pipeline = actions.prepare_packets(ctx.clone())?;
        self.prepare_pipeline(&prepare_pipeline, timeout).await?;

        // Get local pipeline of local task
        let schema = root_actions.get_schema()?;
        let mut root_pipeline =
            self.build_root_pipeline(ctx.get_id(), root_fragment_id, schema, &prepare_pipeline)?;

        let prepare_channel = actions.prepare_channel(ctx.clone())?;
        self.prepare_channel(prepare_channel, timeout).await?;

        QueryCoordinator::init_pipeline(&mut root_pipeline)?;
        self.execute_pipeline(actions.execute_packets(ctx.clone())?, timeout)
            .await?;

        Ok(root_pipeline)
    }

    fn build_root_pipeline(
        &self,
        query_id: String,
        fragment_id: usize,
        schema: DataSchemaRef,
        executor_packet: &[ExecutorPacket],
    ) -> Result<NewPipeline> {
        let mut pipeline = NewPipeline::create();
        self.get_fragment_source(query_id, fragment_id, schema, &mut pipeline)?;

        for executor_packet in executor_packet {
            if executor_packet.executor == executor_packet.request_executor {
                let mut queries_coordinator = self.queries_coordinator.lock();

                match queries_coordinator.entry(executor_packet.query_id.to_owned()) {
                    Entry::Vacant(_) => Err(ErrorCode::LogicalError(format!(
                        "Already exists query id {:?}",
                        executor_packet.query_id
                    ))),
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().prepare_subscribes_channel(executor_packet)
                    }
                }?;
            }
        }

        Ok(pipeline)
    }

    pub fn get_fragment_sink(
        &self,
        query_id: &str,
        id: usize,
        endpoint: &str,
    ) -> Result<FragmentSender> {
        let queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.get(query_id) {
            None => Err(ErrorCode::LogicalError("Query not exists.")),
            Some(coordinator) => coordinator.get_fragment_sink(id, endpoint),
        }
    }

    pub fn get_fragment_source(
        &self,
        query_id: String,
        fragment_id: usize,
        schema: DataSchemaRef,
        pipeline: &mut NewPipeline,
    ) -> Result<()> {
        let mut queries_coordinator = self.queries_coordinator.lock();

        match queries_coordinator.get_mut(&query_id) {
            None => Err(ErrorCode::LogicalError("Query not exists.")),
            Some(query_coordinator) => {
                query_coordinator.subscribe_fragment(fragment_id, schema, pipeline)
            }
        }
    }
}

struct QueryCoordinator {
    ctx: Arc<QueryContext>,
    query_id: String,
    executor_id: String,
    runtime: Arc<Runtime>,
    request_server_tx: Option<Arc<ExchangeSender>>,
    publish_fragments: HashMap<(String, usize), FragmentSender>,
    subscribe_channel: HashMap<String, Sender<Result<DataPacket>>>,
    subscribe_fragments: HashMap<usize, FragmentReceiver>,
    fragments_coordinator: HashMap<usize, Box<FragmentCoordinator>>,

    shutdown_cause: Arc<Mutex<Option<ErrorCode>>>,
    executor: Option<Arc<PipelineCompleteExecutor>>,
    exchange_senders: Vec<Arc<ExchangeSender>>,
    exchange_receivers: Vec<Arc<ExchangeReceiver>>,
}

impl QueryCoordinator {
    pub fn create(ctx: &Arc<QueryContext>, executor: &ExecutorPacket) -> Result<QueryCoordinator> {
        let mut fragments_coordinator = HashMap::with_capacity(executor.fragments.len());

        for fragment in &executor.fragments {
            fragments_coordinator.insert(
                fragment.fragment_id.to_owned(),
                FragmentCoordinator::create(fragment),
            );
        }

        Ok(QueryCoordinator {
            ctx: ctx.clone(),
            fragments_coordinator,
            executor: None,
            exchange_senders: vec![],
            exchange_receivers: vec![],
            shutdown_cause: Arc::new(Mutex::new(None)),
            request_server_tx: None,
            publish_fragments: Default::default(),
            query_id: executor.query_id.to_owned(),
            subscribe_channel: Default::default(),
            subscribe_fragments: Default::default(),
            executor_id: executor.executor.to_owned(),
            runtime: Arc::new(Runtime::with_worker_threads(
                2,
                Some(String::from("Cluster-Pub-Sub")),
            )?),
        })
    }

    pub fn prepare_pipeline(&mut self) -> Result<()> {
        let fragments_id = self
            .fragments_coordinator
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for fragment_id in fragments_id {
            if let Some(coordinator) = self.fragments_coordinator.get_mut(&fragment_id) {
                coordinator.prepare_pipeline(&self.ctx)?;
            }
        }

        Ok(())
    }

    pub fn shutdown(&mut self, cause: Option<ErrorCode>) -> Result<()> {
        {
            let mut shutdown_cause = self.shutdown_cause.lock();
            *shutdown_cause = cause;
        }

        if let Some(executor) = &self.executor {
            executor.finish()?;
        }

        Ok(())
    }

    pub fn prepare_subscribes_channel(&mut self, prepare: &ExecutorPacket) -> Result<()> {
        for source in prepare.source_2_fragments.keys() {
            if source != &prepare.executor {
                let (tx, rx) = async_channel::bounded(1);

                let ctx = self.ctx.clone();
                let query_id = prepare.query_id.clone();
                let runtime = self.runtime.clone();
                let receivers_map = &self.subscribe_fragments;
                let receiver = ExchangeReceiver::create(ctx, query_id, runtime, rx, receivers_map);

                receiver.listen()?;
                self.exchange_receivers.push(receiver);
                self.subscribe_channel.insert(source.to_string(), tx);
            }
        }

        Ok(())
    }

    pub fn execute_pipeline(&mut self) -> Result<()> {
        if self.fragments_coordinator.is_empty() {
            // Empty fragments if it is a request server, because the pipelines may have been linked.
            return Ok(());
        }

        let max_threads = self.ctx.get_settings().get_max_threads()?;
        let mut pipelines = Vec::with_capacity(self.fragments_coordinator.len());

        let mut params = Vec::with_capacity(self.fragments_coordinator.len());
        for coordinator in self.fragments_coordinator.values() {
            params.push(coordinator.create_exchange_params(self)?);
        }

        for ((_, coordinator), params) in self.fragments_coordinator.iter_mut().zip(params) {
            if let Some(mut pipeline) = coordinator.pipeline.take() {
                pipeline.set_max_threads(max_threads as usize);

                if !pipeline.is_pulling_pipeline()? {
                    return Err(ErrorCode::LogicalError("Logical error, It's a bug"));
                }

                // Add exchange data publisher.
                ExchangeSink::publisher_sink(&self.ctx, &params, &mut pipeline)?;

                if !pipeline.is_complete_pipeline()? {
                    return Err(ErrorCode::LogicalError("Logical error, It's a bug"));
                }

                QueryCoordinator::init_pipeline(&mut pipeline)?;
                pipelines.push(pipeline);
            }
        }

        let async_runtime = self.ctx.get_storage_runtime();
        let query_need_abort = self.ctx.query_need_abort();
        let executor =
            PipelineCompleteExecutor::from_pipelines(async_runtime, query_need_abort, pipelines)?;
        self.executor = Some(executor.clone());
        let receivers = self.exchange_receivers.clone();
        let shutdown_cause = self.shutdown_cause.clone();
        let mut request_server_tx = self.request_server_tx.take();

        Thread::named_spawn(Some(String::from("Distributed-Executor")), move || {
            if let Err(cause) = executor.execute() {
                if let Some(request_server_tx) = request_server_tx.take() {
                    if let Err(_cause) = futures::executor::block_on(async move {
                        request_server_tx.send(DataPacket::ErrorCode(cause)).await
                    }) {
                        common_tracing::tracing::warn!("Cannot send error code to request server.");
                    }
                }
            }

            if let Some(cause) = shutdown_cause.lock().take() {
                if let Some(request_server_tx) = request_server_tx.take() {
                    if let Err(_cause) = futures::executor::block_on(async move {
                        request_server_tx.send(DataPacket::ErrorCode(cause)).await
                    }) {
                        common_tracing::tracing::warn!("Cannot send error code to request server.");
                    }
                }
            }

            for receiver in &receivers {
                receiver.shutdown();
            }

            for receiver in &receivers {
                let receiver = receiver.clone();
                futures::executor::block_on(async move {
                    if let Err(cause) = receiver.join().await {
                        common_tracing::tracing::warn!("Receiver join failure {:?}", cause);
                    }
                });
            }
        });

        Ok(())
    }

    pub fn init_publisher(&mut self, senders: HashMap<String, ExchangeSender>) -> Result<()> {
        for (target, exchange_sender) in senders.into_iter() {
            let exchange_sender = Arc::new(exchange_sender);
            let fragments_sender = exchange_sender.listen(self.executor_id.clone())?;

            for (fragment_id, fragment_sender) in fragments_sender.into_iter() {
                self.publish_fragments.insert((target.clone(), fragment_id), fragment_sender);
            }

            if exchange_sender.is_to_request_server() {
                self.request_server_tx = Some(exchange_sender.clone());
            }

            self.exchange_senders.push(exchange_sender);
        }

        for coordinator in self.fragments_coordinator.values_mut() {
            if let Some(pipeline) = coordinator.pipeline.as_mut() {
                Self::init_pipeline(pipeline)?;
            }
        }

        Ok(())
    }

    pub fn init_pipeline(pipeline: &mut NewPipeline) -> Result<()> {
        for pipe in &mut pipeline.pipes {
            if let NewPipe::SimplePipe { processors, .. } = pipe {
                for processor in processors {
                    ExchangeSink::init(processor)?;
                }
            }
        }

        Ok(())
    }

    pub fn receive_data(
        &mut self,
        source: &str,
        mut stream: Streaming<FlightData>,
    ) -> Result<JoinHandle<()>> {
        if let Some(subscribe_channel) = self.subscribe_channel.remove(source) {
            let source = source.to_string();
            let target = self.executor_id.clone();
            return Ok(self.runtime.spawn(async move {
                'fragment_loop: while let Some(flight_data) = stream.next().await {
                    let data_packet = match flight_data {
                        Err(status) => DataPacket::ErrorCode(ErrorCode::from(status)),
                        Ok(flight_data) => match DataPacket::from_flight(flight_data) {
                            Ok(data_packet) => data_packet,
                            Err(error_code) => DataPacket::ErrorCode(error_code),
                        },
                    };

                    if let Err(_cause) = subscribe_channel.send(Ok(data_packet)).await {
                        common_tracing::tracing::warn!(
                            "Subscribe channel closed, source {}, target {}",
                            source,
                            target
                        );

                        break 'fragment_loop;
                    }
                }
            }));
        };

        Err(ErrorCode::LogicalError(
            "Cannot found fragment channel, It's a bug.",
        ))
    }

    async fn create_client(config: Config, address: &str) -> Result<FlightClient> {
        return match config.tls_query_cli_enabled() {
            true => Ok(FlightClient::new(FlightServiceClient::new(
                ConnectionFactory::create_rpc_channel(
                    address.to_owned(),
                    None,
                    Some(config.query.to_rpc_client_tls_config()),
                )
                    .await?,
            ))),
            false => Ok(FlightClient::new(FlightServiceClient::new(
                ConnectionFactory::create_rpc_channel(address.to_owned(), None, None).await?,
            ))),
        };
    }

    fn spawn_pub_worker(&mut self, config: Config, addr: String) -> Result<Sender<DataPacket>> {
        let ctx = self.ctx.clone();
        let query_id = self.query_id.clone();
        let source = self.executor_id.clone();
        let (tx, rx) = async_channel::bounded(2);

        self.runtime.spawn(async move {
            let mut connection = Self::create_client(config, &addr).await?;

            if let Err(status) = connection.do_put(&query_id, &source, rx).await {
                common_tracing::tracing::warn!("Flight connection failure: {:?}", status);

                // Shutdown all query fragments executor and report error to request server.
                let exchange_manager = ctx.get_exchange_manager();
                if let Err(cause) = exchange_manager.shutdown_query(&query_id, Some(status)) {
                    common_tracing::tracing::warn!("Cannot shutdown query, cause {:?}", cause);
                }
            }

            common_exception::Result::Ok(())
        });

        Ok(tx)
    }

    pub fn get_fragment_sink(&self, id: usize, endpoint: &str) -> Result<FragmentSender> {
        match self.publish_fragments.get(&(endpoint.to_string(), id)) {
            None => Err(ErrorCode::LogicalError(format!(
                "Not found node {} in cluster",
                endpoint
            ))),
            Some(publisher) => Ok(publisher.clone()),
        }
    }

    pub fn subscribe_fragment(
        &mut self,
        fragment_id: usize,
        schema: DataSchemaRef,
        pipeline: &mut NewPipeline,
    ) -> Result<()> {
        let (tx, rx) = async_channel::bounded(1);

        // Register subscriber for data exchange.
        self.subscribe_fragments
            .insert(fragment_id, FragmentReceiver::create_unrecorded(tx));

        // Merge pipelines if exist locally pipeline
        if let Some(mut fragment_coordinator) = self.fragments_coordinator.remove(&fragment_id) {
            fragment_coordinator.prepare_pipeline(&self.ctx)?;

            if fragment_coordinator.pipeline.is_none() {
                return Err(ErrorCode::LogicalError(
                    "Pipeline is none, maybe query fragment circular dependency.",
                ));
            }

            if fragment_coordinator.data_exchange.is_none() {
                // When the root fragment and the data has been send to the coordination node,
                // we do not need to wait for the data of other nodes.
                *pipeline = fragment_coordinator.pipeline.unwrap();
                return Ok(());
            }

            let exchange_params = fragment_coordinator.create_exchange_params(self)?;
            *pipeline = fragment_coordinator.pipeline.unwrap();

            // Add exchange data publisher.
            ExchangeSink::via_exchange(&self.ctx, &exchange_params, pipeline)?;

            // Add exchange data subscriber.
            return ExchangeSource::via_exchange(rx, &exchange_params, pipeline);
        }

        // Add exchange data subscriber.
        ExchangeSource::create_source(rx, schema, pipeline)
    }
}

struct FragmentCoordinator {
    node: PlanNode,
    initialized: bool,
    fragment_id: usize,
    data_exchange: Option<DataExchange>,
    pipeline: Option<NewPipeline>,
}

impl FragmentCoordinator {
    pub fn create(packet: &FragmentPacket) -> Box<FragmentCoordinator> {
        Box::new(FragmentCoordinator {
            initialized: false,
            node: packet.node.clone(),
            fragment_id: packet.fragment_id,
            data_exchange: packet.data_exchange.clone(),
            pipeline: None,
        })
    }

    pub fn create_exchange_params(&self, query: &QueryCoordinator) -> Result<ExchangeParams> {
        match &self.data_exchange {
            None => Err(ErrorCode::LogicalError("Cannot found data exchange.")),
            Some(DataExchange::Merge(exchange)) => {
                Ok(ExchangeParams::MergeExchange(MergeExchangeParams {
                    schema: self.node.schema(),
                    fragment_id: self.fragment_id,
                    query_id: query.query_id.to_string(),
                    destination_id: exchange.destination_id.clone(),
                }))
            }
            Some(DataExchange::ShuffleDataExchange(exchange)) => {
                Ok(ExchangeParams::ShuffleExchange(ShuffleExchangeParams {
                    schema: self.node.schema(),
                    fragment_id: self.fragment_id,
                    query_id: query.query_id.to_string(),
                    executor_id: query.executor_id.to_string(),
                    destination_ids: exchange.destination_ids.to_owned(),
                    shuffle_scatter: Arc::new(Box::new(HashFlightScatter::try_create(
                        query.ctx.clone(),
                        self.node.schema(),
                        Some(exchange.exchange_expression.clone()),
                        exchange.destination_ids.len(),
                    )?)),
                }))
            }
        }
    }

    pub fn prepare_pipeline(&mut self, ctx: &Arc<QueryContext>) -> Result<()> {
        if !self.initialized {
            self.initialized = true;
            let pipeline_builder = QueryPipelineBuilder::create(ctx.clone());
            self.pipeline = Some(pipeline_builder.finalize(&self.node)?);
        }

        Ok(())
    }
}
