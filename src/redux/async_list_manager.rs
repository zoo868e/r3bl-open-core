/*
 Copyright 2022 R3BL LLC

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

      https://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  list: SafeList<T>,
}

pub type SafeList<T> = Arc<RwLock<Vec<T>>>;

impl<T> Default for SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  fn default() -> Self {
    Self {
      list: Default::default(),
    }
  }
}

impl<T> SafeListManager<T>
where
  T: Sync + Send + 'static,
{
  pub fn get(&self) -> SafeList<T> {
    self.list.clone()
  }

  pub async fn push(
    &mut self,
    item: T,
  ) {
    let arc = self.get();
    let mut locked_list = arc.write().await;
    locked_list.push(item);
  }

  pub async fn clear(&mut self) {
    let arc = self.get();
    let mut locked_list = arc.write().await;
    locked_list.clear();
  }
}