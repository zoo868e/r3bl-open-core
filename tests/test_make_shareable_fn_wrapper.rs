/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! # Watch macro expansion
//!
//! To watch for changes run this script:
//! `./cargo-watch-macro-expand-one-test.fish test_make_shareable_fn_wrapper`
//!
//! # Watch test output
//!
//! To watch for test output run this script:
//! `./cargo-watch-one-test.fish test_make_shareable_fn_wrapper`

use r3bl_rs_utils_macro::make_shareable_fn_wrapper;

#[test]
fn test_simple_macro_expansion() {
  #![allow(deprecated)]
  make_shareable_fn_wrapper! {
    named FnWrapper1<A>
    containing fn_mut
    of_type FnMut(A) -> Option<A>
  }
}

#[test]
fn test_simple_macro_expansion_2() {
  #![allow(deprecated)]
  make_shareable_fn_wrapper! {
    named FnWrapper2<A>
    containing fn_mut
    of_type Fn(A) -> Option<A>
  }
}

#[test]
fn test_simple_macro_expansion_3() {
  #![allow(deprecated)]
  make_shareable_fn_wrapper! {
    named FnWrapper3<A, B>
    containing fn_mut
    of_type Fn(A) -> Option<B>
  }
}

#[test]
fn test_simple_macro_expansion_4() {
  #![allow(deprecated)]
  make_shareable_fn_wrapper! {
    named FnWrapper4<S, A>
    containing fn_mut
    of_type Fn(&S, &A) -> S
  }
}
