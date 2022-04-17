// Copyright 2021 Datafuse Labs.
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

use common_base::tokio;
use common_exception::Result;
use databend_query::storages::system::TablesTable;
use databend_query::storages::ToReadDataSourcePlan;
use futures::TryStreamExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_tables_table() -> Result<()> {
    let ctx = crate::tests::create_query_context().await?;
    let table = TablesTable::create(1);
    let source_plan = table.read_plan(ctx.clone(), None).await?;

    let stream = table.read(ctx, &source_plan).await?;
    let result = stream.try_collect::<Vec<_>>().await?;
    let block = &result[0];
    assert_eq!(block.num_columns(), 8);

    let expected = vec![
        r"\+--------------------\+--------------\+--------------------\+-------------------------------\+----------\+-----------\+----------------------\+------------\+",
        r"\| database           \| name         \| engine             \| created_on                    \| num_rows \| data_size \| data_compressed_size \| index_size \|",
        r"\+--------------------\+--------------\+--------------------\+-------------------------------\+----------\+-----------\+----------------------\+------------\+",
        r"\| INFORMATION_SCHEMA \| COLUMNS      \| VIEW               \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| INFORMATION_SCHEMA \| KEYWORDS     \| VIEW               \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| INFORMATION_SCHEMA \| SCHEMATA     \| VIEW               \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| INFORMATION_SCHEMA \| TABLES       \| VIEW               \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| INFORMATION_SCHEMA \| VIEWS        \| VIEW               \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| clusters     \| SystemClusters     \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| columns      \| SystemColumns      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| configs      \| SystemConfigs      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| contributors \| SystemContributors \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| credits      \| SystemCredits      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| databases    \| SystemDatabases    \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| engines      \| SystemEngines      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| functions    \| SystemFunctions    \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| metrics      \| SystemMetrics      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| one          \| SystemOne          \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| processes    \| SystemProcesses    \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| query_log    \| SystemQueryLog     \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| roles        \| SystemRoles        \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| settings     \| SystemSettings     \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| tables       \| SystemTables       \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| tracing      \| SystemTracing      \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| users        \| SystemUsers        \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\| system             \| warehouses   \| SystemWarehouses   \| \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3} [\+-]\d{4} \| NULL     \| NULL      \| NULL                 \| NULL       \|",
        r"\+--------------------\+--------------\+--------------------\+-------------------------------\+----------\+-----------\+----------------------\+------------\+",
    ];
    common_datablocks::assert_blocks_sorted_eq_with_regex(expected, result.as_slice());

    Ok(())
}
