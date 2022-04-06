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

use std::sync::Arc;

use common_arrow::arrow::bitmap::MutableBitmap;
use common_exception::Result;
use common_io::prelude::BinaryRead;

use crate::ColumnRef;
use crate::NullableColumn;
use crate::TypeDeserializer;

pub struct NullableDeserializer {
    pub inner: Box<dyn TypeDeserializer>,
    pub bitmap: MutableBitmap,
}

impl TypeDeserializer for NullableDeserializer {
    fn de_binary(&mut self, reader: &mut &[u8]) -> Result<()> {
        let valid: bool = reader.read_scalar()?;
        if valid {
            self.inner.de_binary(reader)?;
        } else {
            self.inner.de_default();
        }
        self.bitmap.push(valid);
        Ok(())
    }

    fn de_default(&mut self) {
        self.inner.de_default();
        self.bitmap.push(false);
    }

    fn de_fixed_binary_batch(&mut self, _reader: &[u8], _step: usize, _rows: usize) -> Result<()> {
        unreachable!()
    }

    fn de_json(&mut self, value: &serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::Null => {
                self.de_null();
                Ok(())
            }
            other => {
                self.bitmap.push(true);
                self.inner.de_json(other)
            }
        }
    }

    // TODO: support null text setting
    fn de_text(&mut self, reader: &[u8]) -> Result<()> {
        self.inner.de_text(reader)?;
        self.bitmap.push(true);
        Ok(())
    }

    fn de_null(&mut self) -> bool {
        self.inner.de_default();
        self.bitmap.push(false);
        true
    }
    fn finish_to_column(&mut self) -> ColumnRef {
        let inner_column = self.inner.finish_to_column();
        let bitmap = std::mem::take(&mut self.bitmap);
        Arc::new(NullableColumn::new(inner_column, bitmap.into()))
    }
}
