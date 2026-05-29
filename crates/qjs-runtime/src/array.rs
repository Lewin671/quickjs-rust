mod constructor;
mod indexing;
mod install;
mod iteration;
mod join;
mod mutation;
mod search;
mod sequence;
mod sort;
mod splice;

pub(crate) use constructor::{native_array, native_array_is_array};
pub(crate) use install::install_array;
pub(crate) use iteration::{
    native_array_prototype_every, native_array_prototype_filter, native_array_prototype_find,
    native_array_prototype_find_index, native_array_prototype_find_last,
    native_array_prototype_find_last_index, native_array_prototype_for_each,
    native_array_prototype_map, native_array_prototype_reduce, native_array_prototype_reduce_right,
    native_array_prototype_some,
};
pub(crate) use join::{native_array_prototype_join, native_array_prototype_to_string};
pub(crate) use mutation::{
    native_array_prototype_copy_within, native_array_prototype_fill, native_array_prototype_pop,
    native_array_prototype_push, native_array_prototype_reverse, native_array_prototype_shift,
    native_array_prototype_unshift,
};
pub(crate) use search::{
    native_array_prototype_at, native_array_prototype_includes, native_array_prototype_index_of,
    native_array_prototype_last_index_of,
};
pub(crate) use sequence::{native_array_prototype_concat, native_array_prototype_slice};
pub(crate) use sort::native_array_prototype_sort;
pub(crate) use splice::native_array_prototype_splice;
