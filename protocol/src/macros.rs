// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[doc(hidden)]
#[macro_export]
macro_rules! create_ids {
    ($t:ty, ) => ();
    ($t:ty, prev($prev:ident), $name:ident) => (
        #[allow(non_upper_case_globals)]
        pub const $name: $t = $prev + 1;
    );
    ($t:ty, prev($prev:ident), $name:ident, $($n:ident),+) => (
        #[allow(non_upper_case_globals)]
        pub const $name: $t = $prev + 1;
        create_ids!($t, prev($name), $($n),+);
    );
    ($t:ty, $name:ident, $($n:ident),+) => (
        #[allow(non_upper_case_globals)]
        pub const $name: $t = 0;
        create_ids!($t, prev($name), $($n),+);
    );
    ($t:ty, $name:ident) => (
        #[allow(non_upper_case_globals)]
        pub const $name: $t = 0;
    );
}
