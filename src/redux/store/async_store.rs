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

use r3bl_rs_utils_macro::make_struct_safe_to_share_and_mutate;
use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio::{spawn, sync::RwLock, task::JoinHandle};

use crate::redux::{
  sync_reducers::ShareableReducerFn, AsyncMiddleware, AsyncSubscriber, StoreStateMachine,
};

make_struct_safe_to_share_and_mutate! {
  named Store<S, A>
  where S: Sync + Send + 'static + Default, A: Sync + Send + 'static + Default
  containing my_store_state_machine
  of_type StoreStateMachine<S, A>
}

/// Thread safe and async Redux store (using [`tokio`]). This is built atop [`StoreData`] (which
/// should not be used directly).
impl<S, A> Store<S, A>
where
  S: Default + Clone + PartialEq + Debug + Hash + Sync + Send + 'static,
  A: Default + Clone + Sync + Send + 'static,
{
  pub async fn get_state(&self) -> S {
    self
      .get_value()
      .await
      .state
      .clone()
  }

  pub async fn get_history(&self) -> Vec<S> {
    self
      .get_value()
      .await
      .history
      .clone()
  }

  pub async fn dispatch_spawn(
    &self,
    action: A,
  ) -> JoinHandle<()> {
    let my_ref = self.get_ref();
    spawn(async move {
      my_ref
        .write()
        .await
        .dispatch_action(action, my_ref.clone())
        .await;
    })
  }

  pub async fn dispatch(
    &self,
    action: A,
  ) {
    let my_ref = self.get_ref();
    my_ref
      .write()
      .await
      .dispatch_action(action.clone(), my_ref.clone())
      .await;
  }

  pub async fn add_subscriber(
    &mut self,
    subscriber_fn: Arc<RwLock<dyn AsyncSubscriber<S> + Send + Sync>>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .subscriber_vec
      .push(subscriber_fn);
    self
  }

  pub async fn clear_subscribers(&mut self) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .subscriber_vec
      .clear();
    self
  }

  pub async fn add_middleware(
    &mut self,
    middleware_fn: Arc<RwLock<dyn AsyncMiddleware<S, A> + Send + Sync>>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .middleware_vec
      .push(middleware_fn);
    self
  }

  pub async fn clear_middlewares(&mut self) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .middleware_vec
      .clear();
    self
  }

  // FIXME: deprecate this w/ new sync trait
  pub async fn add_reducer(
    &mut self,
    reducer_fn: ShareableReducerFn<S, A>,
  ) -> &mut Store<S, A> {
    self
      .get_ref()
      .write()
      .await
      .reducer_fn_list
      .push(reducer_fn)
      .await;
    self
  }
}