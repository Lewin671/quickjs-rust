//! Exhaustive installation-time binding; no operation selection occurs on call.

use super::*;

pub(super) fn bind(native: NativeFunction) -> NativeEntry {
    match native {
        NativeFunction::UninitializedLexical => uninitialized_lexical,
        NativeFunction::Array => arrays::array,
        NativeFunction::ArrayFrom => arrays::array_from,
        NativeFunction::ArrayFromAsync => arrays::array_from_async,
        NativeFunction::ArrayFromAsyncArrayLikeMappedFulfilled => {
            arrays::array_from_async_array_like_mapped_fulfilled
        }
        NativeFunction::ArrayFromAsyncArrayLikeValueFulfilled => {
            arrays::array_from_async_array_like_value_fulfilled
        }
        NativeFunction::ArrayFromAsyncIteratorMappedFulfilled => {
            arrays::array_from_async_iterator_mapped_fulfilled
        }
        NativeFunction::ArrayFromAsyncIteratorRejected => {
            arrays::array_from_async_iterator_rejected
        }
        NativeFunction::ArrayFromAsyncIteratorStepFulfilled => {
            arrays::array_from_async_iterator_step_fulfilled
        }
        NativeFunction::ArrayFromAsyncRejected => arrays::array_from_async_rejected,
        NativeFunction::ArrayIsArray => arrays::array_is_array,
        NativeFunction::ArrayOf => arrays::array_of,
        NativeFunction::ArrayPrototypeAt => arrays::array_prototype_at,
        NativeFunction::ArrayPrototypeConcat => arrays::array_prototype_concat,
        NativeFunction::ArrayPrototypeCopyWithin => arrays::array_prototype_copy_within,
        NativeFunction::ArrayPrototypeEntries => arrays::array_prototype_entries,
        NativeFunction::ArrayPrototypeEvery => arrays::array_prototype_every,
        NativeFunction::ArrayPrototypeFill => arrays::array_prototype_fill,
        NativeFunction::ArrayPrototypeFlat => arrays::array_prototype_flat,
        NativeFunction::ArrayPrototypeFlatMap => arrays::array_prototype_flat_map,
        NativeFunction::ArrayPrototypeFilter => arrays::array_prototype_filter,
        NativeFunction::ArrayPrototypeFind => arrays::array_prototype_find,
        NativeFunction::ArrayPrototypeFindIndex => arrays::array_prototype_find_index,
        NativeFunction::ArrayPrototypeFindLast => arrays::array_prototype_find_last,
        NativeFunction::ArrayPrototypeFindLastIndex => arrays::array_prototype_find_last_index,
        NativeFunction::ArrayPrototypeForEach => arrays::array_prototype_for_each,
        NativeFunction::ArrayPrototypeIncludes => arrays::array_prototype_includes,
        NativeFunction::ArrayPrototypeIndexOf => arrays::array_prototype_index_of,
        NativeFunction::ArrayPrototypeKeys => arrays::array_prototype_keys,
        NativeFunction::ArrayPrototypeLastIndexOf => arrays::array_prototype_last_index_of,
        NativeFunction::ArrayPrototypeJoin => arrays::array_prototype_join,
        NativeFunction::ArrayPrototypeMap => arrays::array_prototype_map,
        NativeFunction::ArrayPrototypePop => arrays::array_prototype_pop,
        NativeFunction::ArrayPrototypePush => arrays::array_prototype_push,
        NativeFunction::ArrayPrototypeReduce => arrays::array_prototype_reduce,
        NativeFunction::ArrayPrototypeReduceRight => arrays::array_prototype_reduce_right,
        NativeFunction::ArrayPrototypeReverse => arrays::array_prototype_reverse,
        NativeFunction::ArrayPrototypeShift => arrays::array_prototype_shift,
        NativeFunction::ArrayPrototypeSlice => arrays::array_prototype_slice,
        NativeFunction::ArrayPrototypeSome => arrays::array_prototype_some,
        NativeFunction::ArrayPrototypeSort => arrays::array_prototype_sort,
        NativeFunction::ArrayPrototypeSplice => arrays::array_prototype_splice,
        NativeFunction::ArrayPrototypeToLocaleString => arrays::array_prototype_to_locale_string,
        NativeFunction::ArrayPrototypeToString => arrays::array_prototype_to_string,
        NativeFunction::ArrayPrototypeToReversed => arrays::array_prototype_to_reversed,
        NativeFunction::ArrayPrototypeToSpliced => arrays::array_prototype_to_spliced,
        NativeFunction::ArrayPrototypeToSorted => arrays::array_prototype_to_sorted,
        NativeFunction::ArrayPrototypeUnshift => arrays::array_prototype_unshift,
        NativeFunction::ArrayPrototypeWith => arrays::array_prototype_with,
        NativeFunction::ArrayPrototypeValues => arrays::array_prototype_values,
        NativeFunction::ArrayIteratorPrototypeNext => arrays::array_iterator_prototype_next,
        NativeFunction::ArrayBuffer => array_buffers::array_buffer,
        NativeFunction::ArrayBufferIsView => array_buffers::array_buffer_is_view,
        NativeFunction::ArrayBufferPrototypeByteLength => {
            array_buffers::array_buffer_prototype_byte_length
        }
        NativeFunction::ArrayBufferPrototypeMaxByteLength => {
            array_buffers::array_buffer_prototype_max_byte_length
        }
        NativeFunction::ArrayBufferPrototypeResizable => {
            array_buffers::array_buffer_prototype_resizable
        }
        NativeFunction::ArrayBufferPrototypeDetached => {
            array_buffers::array_buffer_prototype_detached
        }
        NativeFunction::ArrayBufferPrototypeImmutable => {
            array_buffers::array_buffer_prototype_immutable
        }
        NativeFunction::ArrayBufferPrototypeResize => array_buffers::array_buffer_prototype_resize,
        NativeFunction::ArrayBufferPrototypeSlice => array_buffers::array_buffer_prototype_slice,
        NativeFunction::ArrayBufferPrototypeSliceToImmutable => {
            array_buffers::array_buffer_prototype_slice_to_immutable
        }
        NativeFunction::ArrayBufferPrototypeTransfer => {
            array_buffers::array_buffer_prototype_transfer
        }
        NativeFunction::ArrayBufferPrototypeTransferToFixedLength => {
            array_buffers::array_buffer_prototype_transfer_to_fixed_length
        }
        NativeFunction::ArrayBufferPrototypeTransferToImmutable => {
            array_buffers::array_buffer_prototype_transfer_to_immutable
        }
        NativeFunction::SharedArrayBuffer => array_buffers::shared_array_buffer,
        NativeFunction::SharedArrayBufferPrototypeByteLength => {
            array_buffers::shared_array_buffer_prototype_byte_length
        }
        NativeFunction::SharedArrayBufferPrototypeMaxByteLength => {
            array_buffers::shared_array_buffer_prototype_max_byte_length
        }
        NativeFunction::SharedArrayBufferPrototypeGrowable => {
            array_buffers::shared_array_buffer_prototype_growable
        }
        NativeFunction::SharedArrayBufferPrototypeGrow => {
            array_buffers::shared_array_buffer_prototype_grow
        }
        NativeFunction::SharedArrayBufferPrototypeSlice => {
            array_buffers::shared_array_buffer_prototype_slice
        }
        NativeFunction::AtomicsAdd => atomics::atomics_add,
        NativeFunction::AtomicsAnd => atomics::atomics_and,
        NativeFunction::AtomicsCompareExchange => atomics::atomics_compare_exchange,
        NativeFunction::AtomicsExchange => atomics::atomics_exchange,
        NativeFunction::AtomicsIsLockFree => atomics::atomics_is_lock_free,
        NativeFunction::AtomicsLoad => atomics::atomics_load,
        NativeFunction::AtomicsNotify => atomics::atomics_notify,
        NativeFunction::AtomicsOr => atomics::atomics_or,
        NativeFunction::AtomicsPause => atomics::atomics_pause,
        NativeFunction::AtomicsStore => atomics::atomics_store,
        NativeFunction::AtomicsSub => atomics::atomics_sub,
        NativeFunction::AtomicsWait => atomics::atomics_wait,
        NativeFunction::AtomicsXor => atomics::atomics_xor,
        #[cfg(feature = "agents")]
        NativeFunction::AgentStart => agent::agent_start,
        #[cfg(feature = "agents")]
        NativeFunction::AgentBroadcast => agent::agent_broadcast,
        #[cfg(feature = "agents")]
        NativeFunction::AgentGetReport => agent::agent_get_report,
        #[cfg(feature = "agents")]
        NativeFunction::AgentReport => agent::agent_report,
        #[cfg(feature = "agents")]
        NativeFunction::AgentSleep => agent::agent_sleep,
        #[cfg(feature = "agents")]
        NativeFunction::AgentMonotonicNow => agent::agent_monotonic_now,
        #[cfg(feature = "agents")]
        NativeFunction::AgentReceiveBroadcast => agent::agent_receive_broadcast,
        #[cfg(feature = "agents")]
        NativeFunction::AgentLeaving => agent::agent_leaving,
        NativeFunction::TypedArray => typed_arrays::typed_array,
        NativeFunction::TypedArrayFrom => typed_arrays::typed_array_from,
        NativeFunction::TypedArrayOf => typed_arrays::typed_array_of,
        NativeFunction::TypedArrayPrototypeBuffer => typed_arrays::typed_array_prototype_buffer,
        NativeFunction::TypedArrayPrototypeByteLength => {
            typed_arrays::typed_array_prototype_byte_length
        }
        NativeFunction::TypedArrayPrototypeByteOffset => {
            typed_arrays::typed_array_prototype_byte_offset
        }
        NativeFunction::TypedArrayPrototypeLength => typed_arrays::typed_array_prototype_length,
        NativeFunction::TypedArrayPrototypeToStringTag => {
            typed_arrays::typed_array_prototype_to_string_tag
        }
        NativeFunction::TypedArrayPrototypeAt => typed_arrays::typed_array_prototype_at,
        NativeFunction::TypedArrayPrototypeIndexOf => typed_arrays::typed_array_prototype_index_of,
        NativeFunction::TypedArrayPrototypeLastIndexOf => {
            typed_arrays::typed_array_prototype_last_index_of
        }
        NativeFunction::TypedArrayPrototypeIncludes => typed_arrays::typed_array_prototype_includes,
        NativeFunction::TypedArrayPrototypeJoin => typed_arrays::typed_array_prototype_join,
        NativeFunction::TypedArrayPrototypeKeys => typed_arrays::typed_array_prototype_keys,
        NativeFunction::TypedArrayPrototypeValues => typed_arrays::typed_array_prototype_values,
        NativeFunction::TypedArrayPrototypeEntries => typed_arrays::typed_array_prototype_entries,
        NativeFunction::TypedArrayPrototypeForEach => typed_arrays::typed_array_prototype_for_each,
        NativeFunction::TypedArrayPrototypeMap => typed_arrays::typed_array_prototype_map,
        NativeFunction::TypedArrayPrototypeFilter => typed_arrays::typed_array_prototype_filter,
        NativeFunction::TypedArrayPrototypeReduce => typed_arrays::typed_array_prototype_reduce,
        NativeFunction::TypedArrayPrototypeReduceRight => {
            typed_arrays::typed_array_prototype_reduce_right
        }
        NativeFunction::TypedArrayPrototypeSome => typed_arrays::typed_array_prototype_some,
        NativeFunction::TypedArrayPrototypeEvery => typed_arrays::typed_array_prototype_every,
        NativeFunction::TypedArrayPrototypeFind => typed_arrays::typed_array_prototype_find,
        NativeFunction::TypedArrayPrototypeFindIndex => {
            typed_arrays::typed_array_prototype_find_index
        }
        NativeFunction::TypedArrayPrototypeFindLast => {
            typed_arrays::typed_array_prototype_find_last
        }
        NativeFunction::TypedArrayPrototypeFindLastIndex => {
            typed_arrays::typed_array_prototype_find_last_index
        }
        NativeFunction::TypedArrayPrototypeSlice => typed_arrays::typed_array_prototype_slice,
        NativeFunction::TypedArrayPrototypeSubarray => typed_arrays::typed_array_prototype_subarray,
        NativeFunction::TypedArrayPrototypeToString => {
            typed_arrays::typed_array_prototype_to_string
        }
        NativeFunction::TypedArrayPrototypeToLocaleString => {
            typed_arrays::typed_array_prototype_to_locale_string
        }
        NativeFunction::TypedArrayPrototypeSet => typed_arrays::typed_array_prototype_set,
        NativeFunction::TypedArrayPrototypeFill => typed_arrays::typed_array_prototype_fill,
        NativeFunction::TypedArrayPrototypeCopyWithin => {
            typed_arrays::typed_array_prototype_copy_within
        }
        NativeFunction::TypedArrayPrototypeReverse => typed_arrays::typed_array_prototype_reverse,
        NativeFunction::TypedArrayPrototypeSort => typed_arrays::typed_array_prototype_sort,
        NativeFunction::TypedArrayPrototypeToReversed => {
            typed_arrays::typed_array_prototype_to_reversed
        }
        NativeFunction::TypedArrayPrototypeToSorted => {
            typed_arrays::typed_array_prototype_to_sorted
        }
        NativeFunction::TypedArrayPrototypeWith => typed_arrays::typed_array_prototype_with,
        NativeFunction::Uint8ArrayFromBase64 => typed_arrays::uint8_array_from_base64,
        NativeFunction::Uint8ArrayFromHex => typed_arrays::uint8_array_from_hex,
        NativeFunction::Uint8ArrayPrototypeToBase64 => {
            typed_arrays::uint8_array_prototype_to_base64
        }
        NativeFunction::Uint8ArrayPrototypeToHex => typed_arrays::uint8_array_prototype_to_hex,
        NativeFunction::Uint8ArrayPrototypeSetFromBase64 => {
            typed_arrays::uint8_array_prototype_set_from_base64
        }
        NativeFunction::Uint8ArrayPrototypeSetFromHex => {
            typed_arrays::uint8_array_prototype_set_from_hex
        }
        NativeFunction::DataView => data_views::data_view,
        NativeFunction::DataViewPrototypeBuffer => data_views::data_view_prototype_buffer,
        NativeFunction::DataViewPrototypeByteLength => data_views::data_view_prototype_byte_length,
        NativeFunction::DataViewPrototypeByteOffset => data_views::data_view_prototype_byte_offset,
        NativeFunction::DataViewPrototypeGetInt8 => data_views::data_view_prototype_get_int8,
        NativeFunction::DataViewPrototypeGetUint8 => data_views::data_view_prototype_get_uint8,
        NativeFunction::DataViewPrototypeGetInt16 => data_views::data_view_prototype_get_int16,
        NativeFunction::DataViewPrototypeGetUint16 => data_views::data_view_prototype_get_uint16,
        NativeFunction::DataViewPrototypeGetInt32 => data_views::data_view_prototype_get_int32,
        NativeFunction::DataViewPrototypeGetUint32 => data_views::data_view_prototype_get_uint32,
        NativeFunction::DataViewPrototypeGetFloat16 => data_views::data_view_prototype_get_float16,
        NativeFunction::DataViewPrototypeGetFloat32 => data_views::data_view_prototype_get_float32,
        NativeFunction::DataViewPrototypeGetFloat64 => data_views::data_view_prototype_get_float64,
        NativeFunction::DataViewPrototypeGetBigInt64 => {
            data_views::data_view_prototype_get_big_int64
        }
        NativeFunction::DataViewPrototypeGetBigUint64 => {
            data_views::data_view_prototype_get_big_uint64
        }
        NativeFunction::DataViewPrototypeSetInt8 => data_views::data_view_prototype_set_int8,
        NativeFunction::DataViewPrototypeSetUint8 => data_views::data_view_prototype_set_uint8,
        NativeFunction::DataViewPrototypeSetInt16 => data_views::data_view_prototype_set_int16,
        NativeFunction::DataViewPrototypeSetUint16 => data_views::data_view_prototype_set_uint16,
        NativeFunction::DataViewPrototypeSetInt32 => data_views::data_view_prototype_set_int32,
        NativeFunction::DataViewPrototypeSetUint32 => data_views::data_view_prototype_set_uint32,
        NativeFunction::DataViewPrototypeSetFloat16 => data_views::data_view_prototype_set_float16,
        NativeFunction::DataViewPrototypeSetFloat32 => data_views::data_view_prototype_set_float32,
        NativeFunction::DataViewPrototypeSetFloat64 => data_views::data_view_prototype_set_float64,
        NativeFunction::DataViewPrototypeSetBigInt64 => {
            data_views::data_view_prototype_set_big_int64
        }
        NativeFunction::DataViewPrototypeSetBigUint64 => {
            data_views::data_view_prototype_set_big_uint64
        }
        NativeFunction::AsyncDisposableStack => disposable_stacks::async_disposable_stack,
        NativeFunction::AsyncDisposableStackPrototypeAdopt => {
            disposable_stacks::async_disposable_stack_prototype_adopt
        }
        NativeFunction::AsyncDisposableStackPrototypeDefer => {
            disposable_stacks::async_disposable_stack_prototype_defer
        }
        NativeFunction::AsyncDisposableStackPrototypeDisposed => {
            disposable_stacks::async_disposable_stack_prototype_disposed
        }
        NativeFunction::AsyncDisposableStackPrototypeDisposeAsync => {
            disposable_stacks::async_disposable_stack_prototype_dispose_async
        }
        NativeFunction::AsyncDisposableStackPrototypeMove => {
            disposable_stacks::async_disposable_stack_prototype_move
        }
        NativeFunction::AsyncDisposableStackPrototypeUse => {
            disposable_stacks::async_disposable_stack_prototype_use
        }
        NativeFunction::DisposableStack => disposable_stacks::disposable_stack,
        NativeFunction::DisposableStackPrototypeAdopt => {
            disposable_stacks::disposable_stack_prototype_adopt
        }
        NativeFunction::DisposableStackPrototypeDefer => {
            disposable_stacks::disposable_stack_prototype_defer
        }
        NativeFunction::DisposableStackPrototypeDisposed => {
            disposable_stacks::disposable_stack_prototype_disposed
        }
        NativeFunction::DisposableStackPrototypeDispose => {
            disposable_stacks::disposable_stack_prototype_dispose
        }
        NativeFunction::DisposableStackPrototypeMove => {
            disposable_stacks::disposable_stack_prototype_move
        }
        NativeFunction::DisposableStackPrototypeUse => {
            disposable_stacks::disposable_stack_prototype_use
        }
        NativeFunction::Uint8Array => typed_arrays::uint8_array,
        NativeFunction::Int8Array => typed_arrays::int8_array,
        NativeFunction::Uint8ClampedArray => typed_arrays::uint8_clamped_array,
        NativeFunction::Uint16Array => typed_arrays::uint16_array,
        NativeFunction::Int16Array => typed_arrays::int16_array,
        NativeFunction::Uint32Array => typed_arrays::uint32_array,
        NativeFunction::Int32Array => typed_arrays::int32_array,
        NativeFunction::Float32Array => typed_arrays::float32_array,
        NativeFunction::Float64Array => typed_arrays::float64_array,
        NativeFunction::BigInt64Array => typed_arrays::big_int64_array,
        NativeFunction::BigUint64Array => typed_arrays::big_uint64_array,
        NativeFunction::IsHtmlDda => core::is_html_dda,
        NativeFunction::DetachArrayBuffer => array_buffers::detach_array_buffer,
        NativeFunction::EvalScript => core::eval_script,
        NativeFunction::IteratorPrototypeIterator => core::iterator_prototype_iterator,
        NativeFunction::Iterator => crate::iterator::iterator,
        NativeFunction::IteratorFrom => crate::iterator::iterator_from,
        NativeFunction::IteratorConcat => crate::iterator::iterator_concat,
        NativeFunction::IteratorZip => crate::iterator::iterator_zip,
        NativeFunction::IteratorZipKeyed => crate::iterator::iterator_zip_keyed,
        NativeFunction::IteratorPrototypeToStringTagGet => {
            crate::iterator::iterator_prototype_to_string_tag_get
        }
        NativeFunction::IteratorPrototypeToStringTagSet => {
            crate::iterator::iterator_prototype_to_string_tag_set
        }
        NativeFunction::IteratorPrototypeConstructorGet => {
            crate::iterator::iterator_prototype_constructor_get
        }
        NativeFunction::IteratorPrototypeConstructorSet => {
            crate::iterator::iterator_prototype_constructor_set
        }
        NativeFunction::IteratorPrototypeMap => crate::iterator::iterator_prototype_map,
        NativeFunction::IteratorPrototypeFilter => crate::iterator::iterator_prototype_filter,
        NativeFunction::IteratorPrototypeTake => crate::iterator::iterator_prototype_take,
        NativeFunction::IteratorPrototypeDrop => crate::iterator::iterator_prototype_drop,
        NativeFunction::IteratorPrototypeFlatMap => crate::iterator::iterator_prototype_flat_map,
        NativeFunction::IteratorPrototypeReduce => crate::iterator::iterator_prototype_reduce,
        NativeFunction::IteratorPrototypeToArray => crate::iterator::iterator_prototype_to_array,
        NativeFunction::IteratorPrototypeForEach => crate::iterator::iterator_prototype_for_each,
        NativeFunction::IteratorPrototypeSome => crate::iterator::iterator_prototype_some,
        NativeFunction::IteratorPrototypeEvery => crate::iterator::iterator_prototype_every,
        NativeFunction::IteratorPrototypeFind => crate::iterator::iterator_prototype_find,
        NativeFunction::IteratorPrototypeDispose => crate::iterator::iterator_prototype_dispose,
        NativeFunction::IteratorHelperPrototypeNext => {
            crate::iterator::iterator_helper_prototype_next
        }
        NativeFunction::IteratorHelperPrototypeReturn => {
            crate::iterator::iterator_helper_prototype_return
        }
        NativeFunction::WrapForValidIteratorPrototypeNext => {
            crate::iterator::wrap_for_valid_iterator_prototype_next
        }
        NativeFunction::WrapForValidIteratorPrototypeReturn => {
            crate::iterator::wrap_for_valid_iterator_prototype_return
        }
        NativeFunction::Boolean => core::boolean,
        NativeFunction::BooleanPrototypeToString => core::boolean_prototype_to_string,
        NativeFunction::BooleanPrototypeValueOf => core::boolean_prototype_value_of,
        NativeFunction::Date => date::date,
        NativeFunction::DateNow => date::date_now,
        NativeFunction::DateParse => date::date_parse,
        NativeFunction::DatePrototypeGetTimezoneOffset => date::date_prototype_get_timezone_offset,
        NativeFunction::DatePrototypeGetYear => date::date_prototype_get_year,
        NativeFunction::DatePrototypeGetUtcDate => date::date_prototype_get_utc_date,
        NativeFunction::DatePrototypeGetUtcDay => date::date_prototype_get_utc_day,
        NativeFunction::DatePrototypeGetUtcFullYear => date::date_prototype_get_utc_full_year,
        NativeFunction::DatePrototypeGetUtcHours => date::date_prototype_get_utc_hours,
        NativeFunction::DatePrototypeGetUtcMilliseconds => {
            date::date_prototype_get_utc_milliseconds
        }
        NativeFunction::DatePrototypeGetUtcMinutes => date::date_prototype_get_utc_minutes,
        NativeFunction::DatePrototypeGetUtcMonth => date::date_prototype_get_utc_month,
        NativeFunction::DatePrototypeGetUtcSeconds => date::date_prototype_get_utc_seconds,
        NativeFunction::DatePrototypeGetTime => date::date_prototype_get_time,
        NativeFunction::DatePrototypeSetYear => date::date_prototype_set_year,
        NativeFunction::DatePrototypeSetTime => date::date_prototype_set_time,
        NativeFunction::DatePrototypeSetUtcDate => date::date_prototype_set_utc_date,
        NativeFunction::DatePrototypeSetUtcFullYear => date::date_prototype_set_utc_full_year,
        NativeFunction::DatePrototypeSetUtcHours => date::date_prototype_set_utc_hours,
        NativeFunction::DatePrototypeSetUtcMilliseconds => {
            date::date_prototype_set_utc_milliseconds
        }
        NativeFunction::DatePrototypeSetUtcMinutes => date::date_prototype_set_utc_minutes,
        NativeFunction::DatePrototypeSetUtcMonth => date::date_prototype_set_utc_month,
        NativeFunction::DatePrototypeSetUtcSeconds => date::date_prototype_set_utc_seconds,
        NativeFunction::DatePrototypeToDateString => date::date_prototype_to_date_string,
        NativeFunction::DatePrototypeToISOString => date::date_prototype_to_iso_string,
        NativeFunction::DatePrototypeToJson => date::date_prototype_to_json,
        NativeFunction::DatePrototypeToPrimitive => date::date_prototype_to_primitive,
        NativeFunction::DatePrototypeToString => date::date_prototype_to_string,
        NativeFunction::DatePrototypeToTimeString => date::date_prototype_to_time_string,
        NativeFunction::DatePrototypeToUtcString => date::date_prototype_to_utc_string,
        NativeFunction::DatePrototypeValueOf => date::date_prototype_value_of,
        NativeFunction::DateUtc => date::date_utc,
        NativeFunction::Error => errors::error,
        NativeFunction::ErrorIsError => errors::error_is_error,
        NativeFunction::ErrorPrototypeToString => errors::error_prototype_to_string,
        NativeFunction::AggregateError => errors::aggregate_error,
        NativeFunction::SuppressedError => errors::suppressed_error,
        NativeFunction::DecodeUri => core::decode_uri,
        NativeFunction::DecodeUriComponent => core::decode_uri_component,
        NativeFunction::EncodeUri => core::encode_uri,
        NativeFunction::EncodeUriComponent => core::encode_uri_component,
        NativeFunction::EvalError => errors::eval_error,
        NativeFunction::Eval => core::eval,
        NativeFunction::Escape => core::escape,
        NativeFunction::MathAbs => math::math_abs,
        NativeFunction::MathAcos => math::math_acos,
        NativeFunction::MathAcosh => math::math_acosh,
        NativeFunction::MathAsin => math::math_asin,
        NativeFunction::MathAsinh => math::math_asinh,
        NativeFunction::MathAtan => math::math_atan,
        NativeFunction::MathAtan2 => math::math_atan2,
        NativeFunction::MathAtanh => math::math_atanh,
        NativeFunction::MathCbrt => math::math_cbrt,
        NativeFunction::MathCeil => math::math_ceil,
        NativeFunction::MathClz32 => math::math_clz32,
        NativeFunction::MathCos => math::math_cos,
        NativeFunction::MathCosh => math::math_cosh,
        NativeFunction::MathExp => math::math_exp,
        NativeFunction::MathExpm1 => math::math_expm1,
        NativeFunction::MathF16round => math::math_f16round,
        NativeFunction::MathFloor => math::math_floor,
        NativeFunction::MathFround => math::math_fround,
        NativeFunction::MathHypot => math::math_hypot,
        NativeFunction::MathImul => math::math_imul,
        NativeFunction::MathLog => math::math_log,
        NativeFunction::MathLog1p => math::math_log1p,
        NativeFunction::MathLog10 => math::math_log10,
        NativeFunction::MathLog2 => math::math_log2,
        NativeFunction::MathMax => math::math_max,
        NativeFunction::MathMin => math::math_min,
        NativeFunction::MathPow => math::math_pow,
        NativeFunction::MathRandom => math::math_random,
        NativeFunction::MathRound => math::math_round,
        NativeFunction::MathSign => math::math_sign,
        NativeFunction::MathSin => math::math_sin,
        NativeFunction::MathSinh => math::math_sinh,
        NativeFunction::MathSqrt => math::math_sqrt,
        NativeFunction::MathSumPrecise => math::math_sum_precise,
        NativeFunction::MathTan => math::math_tan,
        NativeFunction::MathTanh => math::math_tanh,
        NativeFunction::MathTrunc => math::math_trunc,
        NativeFunction::FunctionPrototype => core::function_prototype,
        NativeFunction::GeneratorFunction => core::generator_function,
        NativeFunction::GeneratorPrototypeNext => crate::generator::generator_prototype_next,
        NativeFunction::GeneratorPrototypeReturn => crate::generator::generator_prototype_return,
        NativeFunction::GeneratorPrototypeThrow => crate::generator::generator_prototype_throw,
        NativeFunction::AsyncFunction => core::async_function,
        NativeFunction::AsyncGeneratorFunction => core::async_generator_function,
        NativeFunction::AsyncFunctionAwaitFulfilled => {
            crate::async_function::async_function_await_fulfilled
        }
        NativeFunction::AsyncFunctionAwaitRejected => {
            crate::async_function::async_function_await_rejected
        }
        NativeFunction::AsyncGeneratorPrototypeNext => {
            crate::async_generator::async_generator_prototype_next
        }
        NativeFunction::AsyncGeneratorPrototypeReturn => {
            crate::async_generator::async_generator_prototype_return
        }
        NativeFunction::AsyncGeneratorPrototypeThrow => {
            crate::async_generator::async_generator_prototype_throw
        }
        NativeFunction::AsyncGeneratorPrototypeAsyncIterator => {
            crate::async_generator::async_generator_prototype_async_iterator
        }
        NativeFunction::AsyncIteratorPrototypeAsyncDispose => {
            crate::async_generator::async_iterator_prototype_async_dispose
        }
        NativeFunction::AsyncDisposeReturnUndefined => {
            crate::async_generator::async_dispose_return_undefined
        }
        NativeFunction::AsyncGeneratorAwaitFulfilled => {
            crate::async_generator::async_generator_await_fulfilled
        }
        NativeFunction::AsyncGeneratorAwaitRejected => {
            crate::async_generator::async_generator_await_rejected
        }
        NativeFunction::AsyncGeneratorYieldFulfilled => {
            crate::async_generator::async_generator_yield_fulfilled
        }
        NativeFunction::AsyncGeneratorYieldRejected => {
            crate::async_generator::async_generator_yield_rejected
        }
        NativeFunction::AsyncGeneratorReturnResumeFulfilled => {
            crate::async_generator::async_generator_return_resume_fulfilled
        }
        NativeFunction::AsyncGeneratorReturnResumeRejected => {
            crate::async_generator::async_generator_return_resume_rejected
        }
        NativeFunction::AsyncGeneratorReturnFulfilled => {
            crate::async_generator::async_generator_return_fulfilled
        }
        NativeFunction::AsyncGeneratorReturnRejected => {
            crate::async_generator::async_generator_return_rejected
        }
        NativeFunction::AsyncFromSyncIteratorValueFulfilled => {
            crate::async_generator::async_from_sync_iterator_value_fulfilled
        }
        NativeFunction::AsyncFromSyncIteratorValueRejected => {
            crate::async_generator::async_from_sync_iterator_value_rejected
        }
        NativeFunction::AsyncFromSyncIteratorNext => {
            crate::async_generator::async_from_sync_iterator_next
        }
        NativeFunction::AsyncFromSyncIteratorReturn => {
            crate::async_generator::async_from_sync_iterator_return
        }
        NativeFunction::AsyncFromSyncIteratorThrow => {
            crate::async_generator::async_from_sync_iterator_throw
        }
        NativeFunction::GlobalIsFinite => core::global_is_finite,
        NativeFunction::GlobalIsNaN => core::global_is_na_n,
        NativeFunction::Test262AssertSameValue => core::test262_assert_same_value,
        NativeFunction::Test262VerifyProperty => core::test262_verify_property,
        NativeFunction::Test262BuildString => core::test262_build_string,
        NativeFunction::Test262ToNumbers => core::test262_to_numbers,
        NativeFunction::Test262CompareArray => core::test262_compare_array,
        NativeFunction::Test262AssertIteratorResult => core::test262_assert_iterator_result,
        NativeFunction::Test262AssertPackedArray => core::test262_assert_packed_array,
        NativeFunction::Test262AssertNullProtoMutableObject => {
            core::test262_assert_null_proto_mutable_object
        }
        NativeFunction::Test262AssertNativeFn => core::test262_assert_native_fn,
        NativeFunction::Test262AssertRegExpSourceLoop => core::test262_assert_reg_exp_source_loop,
        NativeFunction::Test262AssertRegExpWhitespaceLoop => {
            core::test262_assert_reg_exp_whitespace_loop
        }
        NativeFunction::Test262AssertUriLoop => core::test262_assert_uri_loop,
        NativeFunction::Test262AssertStringSubstrNumberLoop => {
            core::test262_assert_string_substr_number_loop
        }
        NativeFunction::Test262AssertArrayBufferSliceToImmutableArgumentCoercion => {
            core::test262_assert_array_buffer_slice_to_immutable_argument_coercion
        }
        NativeFunction::Test262AssertLineCommentUnicodeLoop => {
            core::test262_assert_line_comment_unicode_loop
        }
        NativeFunction::Print => core::print,
        NativeFunction::Function => core::function,
        NativeFunction::FunctionPrototypeApply => core::function_prototype_apply,
        NativeFunction::FunctionPrototypeBind => core::function_prototype_bind,
        NativeFunction::FunctionPrototypeCall => core::function_prototype_call,
        NativeFunction::FunctionPrototypeHasInstance => core::function_prototype_has_instance,
        NativeFunction::FunctionPrototypeToString => core::function_prototype_to_string,
        NativeFunction::MappedArgumentGet => core::mapped_argument_get,
        NativeFunction::MappedArgumentSet => core::mapped_argument_set,
        NativeFunction::ThrowTypeError => core::throw_type_error,
        NativeFunction::RealmThrowTypeError => core::realm_throw_type_error,
        NativeFunction::JsonParse => json::json_parse,
        NativeFunction::JsonRawJson => json::json_raw_json,
        NativeFunction::JsonIsRawJson => json::json_is_raw_json,
        NativeFunction::JsonStringify => json::json_stringify,
        NativeFunction::Map => maps::map,
        NativeFunction::MapGroupBy => maps::map_group_by,
        NativeFunction::MapPrototypeClear => maps::map_prototype_clear,
        NativeFunction::MapPrototypeDelete => maps::map_prototype_delete,
        NativeFunction::MapPrototypeEntries => maps::map_prototype_entries,
        NativeFunction::MapPrototypeForEach => maps::map_prototype_for_each,
        NativeFunction::MapPrototypeGet => maps::map_prototype_get,
        NativeFunction::MapPrototypeGetOrInsert => maps::map_prototype_get_or_insert,
        NativeFunction::MapPrototypeGetOrInsertComputed => {
            maps::map_prototype_get_or_insert_computed
        }
        NativeFunction::MapPrototypeHas => maps::map_prototype_has,
        NativeFunction::MapPrototypeKeys => maps::map_prototype_keys,
        NativeFunction::MapPrototypeSet => maps::map_prototype_set,
        NativeFunction::MapPrototypeSize => maps::map_prototype_size,
        NativeFunction::MapPrototypeValues => maps::map_prototype_values,
        NativeFunction::MapIteratorPrototypeNext => maps::map_iterator_prototype_next,
        NativeFunction::WeakMap => weak_maps::weak_map,
        NativeFunction::WeakMapPrototypeDelete => weak_maps::weak_map_prototype_delete,
        NativeFunction::WeakMapPrototypeGet => weak_maps::weak_map_prototype_get,
        NativeFunction::WeakMapPrototypeGetOrInsert => weak_maps::weak_map_prototype_get_or_insert,
        NativeFunction::WeakMapPrototypeGetOrInsertComputed => {
            weak_maps::weak_map_prototype_get_or_insert_computed
        }
        NativeFunction::WeakMapPrototypeHas => weak_maps::weak_map_prototype_has,
        NativeFunction::WeakMapPrototypeSet => weak_maps::weak_map_prototype_set,
        NativeFunction::FinalizationRegistry => finalization_registries::finalization_registry,
        NativeFunction::FinalizationRegistryPrototypeRegister => {
            finalization_registries::finalization_registry_prototype_register
        }
        NativeFunction::FinalizationRegistryPrototypeUnregister => {
            finalization_registries::finalization_registry_prototype_unregister
        }
        NativeFunction::WeakRef => weak_refs::weak_ref,
        NativeFunction::WeakRefPrototypeDeref => weak_refs::weak_ref_prototype_deref,
        NativeFunction::WeakSet => weak_sets::weak_set,
        NativeFunction::WeakSetPrototypeAdd => weak_sets::weak_set_prototype_add,
        NativeFunction::WeakSetPrototypeDelete => weak_sets::weak_set_prototype_delete,
        NativeFunction::WeakSetPrototypeHas => weak_sets::weak_set_prototype_has,
        NativeFunction::Set => sets::set,
        NativeFunction::SetPrototypeAdd => sets::set_prototype_add,
        NativeFunction::SetPrototypeClear => sets::set_prototype_clear,
        NativeFunction::SetPrototypeDelete => sets::set_prototype_delete,
        NativeFunction::SetPrototypeDifference => sets::set_prototype_difference,
        NativeFunction::SetPrototypeEntries => sets::set_prototype_entries,
        NativeFunction::SetPrototypeForEach => sets::set_prototype_for_each,
        NativeFunction::SetPrototypeHas => sets::set_prototype_has,
        NativeFunction::SetPrototypeIntersection => sets::set_prototype_intersection,
        NativeFunction::SetPrototypeIsDisjointFrom => sets::set_prototype_is_disjoint_from,
        NativeFunction::SetPrototypeIsSubsetOf => sets::set_prototype_is_subset_of,
        NativeFunction::SetPrototypeIsSupersetOf => sets::set_prototype_is_superset_of,
        NativeFunction::SetPrototypeSize => sets::set_prototype_size,
        NativeFunction::SetPrototypeSymmetricDifference => sets::set_prototype_symmetric_difference,
        NativeFunction::SetPrototypeUnion => sets::set_prototype_union,
        NativeFunction::SetPrototypeValues => sets::set_prototype_values,
        NativeFunction::SetIteratorPrototypeNext => sets::set_iterator_prototype_next,
        NativeFunction::SpeciesGetter => core::species_getter,
        NativeFunction::BigInt => core::big_int,
        NativeFunction::BigIntAsIntN => core::big_int_as_int_n,
        NativeFunction::BigIntAsUintN => core::big_int_as_uint_n,
        NativeFunction::BigIntPrototypeToLocaleString => core::big_int_prototype_to_locale_string,
        NativeFunction::BigIntPrototypeToString => core::big_int_prototype_to_string,
        NativeFunction::BigIntPrototypeValueOf => core::big_int_prototype_value_of,
        NativeFunction::Number => numbers::number,
        NativeFunction::NumberIsFinite => numbers::number_is_finite,
        NativeFunction::NumberIsInteger => numbers::number_is_integer,
        NativeFunction::NumberIsNaN => numbers::number_is_na_n,
        NativeFunction::NumberIsSafeInteger => numbers::number_is_safe_integer,
        NativeFunction::NumberPrototypeToExponential => numbers::number_prototype_to_exponential,
        NativeFunction::NumberPrototypeToFixed => numbers::number_prototype_to_fixed,
        NativeFunction::NumberPrototypeToLocaleString => numbers::number_prototype_to_locale_string,
        NativeFunction::NumberPrototypeToPrecision => numbers::number_prototype_to_precision,
        NativeFunction::NumberPrototypeToString => numbers::number_prototype_to_string,
        NativeFunction::NumberPrototypeValueOf => numbers::number_prototype_value_of,
        NativeFunction::ParseFloat => numbers::parse_float,
        NativeFunction::ParseInt => numbers::parse_int,
        NativeFunction::Promise => promises::promise,
        NativeFunction::PromiseAll => promises::promise_all,
        NativeFunction::PromiseAny => promises::promise_any,
        NativeFunction::PromiseAnyRejectElement => promises::promise_any_reject_element,
        NativeFunction::PromiseAllSettled => promises::promise_all_settled,
        NativeFunction::PromiseAllSettledRejectElement => {
            promises::promise_all_settled_reject_element
        }
        NativeFunction::PromiseAllSettledResolveElement => {
            promises::promise_all_settled_resolve_element
        }
        NativeFunction::PromiseAllResolveElement => promises::promise_all_resolve_element,
        NativeFunction::PromiseGetCapabilitiesExecutor => {
            promises::promise_get_capabilities_executor
        }
        NativeFunction::PromisePrototypeCatch => promises::promise_prototype_catch,
        NativeFunction::PromisePrototypeFinally => promises::promise_prototype_finally,
        NativeFunction::PromisePrototypeFinallyFulfilled => {
            promises::promise_prototype_finally_fulfilled
        }
        NativeFunction::PromisePrototypeFinallyRejected => {
            promises::promise_prototype_finally_rejected
        }
        NativeFunction::PromisePrototypeFinallyValueThunk => {
            promises::promise_prototype_finally_value_thunk
        }
        NativeFunction::PromisePrototypeFinallyThrowerThunk => {
            promises::promise_prototype_finally_thrower_thunk
        }
        NativeFunction::PromisePrototypeThen => promises::promise_prototype_then,
        NativeFunction::PromiseRace => promises::promise_race,
        NativeFunction::PromiseReject => promises::promise_reject,
        NativeFunction::PromiseResolve => promises::promise_resolve,
        NativeFunction::PromiseRejectFunction => promises::promise_reject_function,
        NativeFunction::PromiseResolveFunction => promises::promise_resolve_function,
        NativeFunction::PromiseTry => promises::promise_try,
        NativeFunction::PromiseWithResolvers => promises::promise_with_resolvers,
        NativeFunction::Proxy => core::proxy,
        NativeFunction::ProxyRevocable => core::proxy_revocable,
        NativeFunction::ProxyRevoke => core::proxy_revoke,
        NativeFunction::RangeError => errors::range_error,
        NativeFunction::ReferenceError => errors::reference_error,
        NativeFunction::Object => objects::object,
        NativeFunction::ObjectAssign => objects::object_assign,
        NativeFunction::ObjectCreate => objects::object_create,
        NativeFunction::ObjectDefineProperties => objects::object_define_properties,
        NativeFunction::ObjectDefineProperty => objects::object_define_property,
        NativeFunction::ObjectGetOwnPropertyDescriptor => {
            objects::object_get_own_property_descriptor
        }
        NativeFunction::ObjectGetOwnPropertyDescriptors => {
            objects::object_get_own_property_descriptors
        }
        NativeFunction::ObjectGetPrototypeOf => objects::object_get_prototype_of,
        NativeFunction::ObjectGetOwnPropertyNames => objects::object_get_own_property_names,
        NativeFunction::ObjectGetOwnPropertySymbols => objects::object_get_own_property_symbols,
        NativeFunction::ObjectFromEntries => objects::object_from_entries,
        NativeFunction::ObjectGroupBy => objects::object_group_by,
        NativeFunction::ObjectHasOwn => objects::object_has_own,
        NativeFunction::ObjectFreeze => objects::object_freeze,
        NativeFunction::ObjectIs => objects::object_is,
        NativeFunction::ObjectIsExtensible => objects::object_is_extensible,
        NativeFunction::ObjectIsFrozen => objects::object_is_frozen,
        NativeFunction::ObjectIsSealed => objects::object_is_sealed,
        NativeFunction::ObjectPreventExtensions => objects::object_prevent_extensions,
        NativeFunction::ObjectSeal => objects::object_seal,
        NativeFunction::ObjectSetPrototypeOf => objects::object_set_prototype_of,
        NativeFunction::ObjectEntries => objects::object_entries,
        NativeFunction::ObjectKeys => objects::object_keys,
        NativeFunction::ObjectValues => objects::object_values,
        NativeFunction::ObjectPrototypeHasOwnProperty => objects::object_prototype_has_own_property,
        NativeFunction::ObjectPrototypeIsPrototypeOf => objects::object_prototype_is_prototype_of,
        NativeFunction::ObjectPrototypePropertyIsEnumerable => {
            objects::object_prototype_property_is_enumerable
        }
        NativeFunction::ObjectPrototypeToString => objects::object_prototype_to_string,
        NativeFunction::ObjectPrototypeToLocaleString => objects::object_prototype_to_locale_string,
        NativeFunction::ObjectPrototypeValueOf => objects::object_prototype_value_of,
        NativeFunction::ObjectPrototypeGetProto => objects::object_prototype_get_proto,
        NativeFunction::ObjectPrototypeSetProto => objects::object_prototype_set_proto,
        NativeFunction::ObjectPrototypeDefineGetter => objects::object_prototype_define_getter,
        NativeFunction::ObjectPrototypeDefineSetter => objects::object_prototype_define_setter,
        NativeFunction::ObjectPrototypeLookupGetter => objects::object_prototype_lookup_getter,
        NativeFunction::ObjectPrototypeLookupSetter => objects::object_prototype_lookup_setter,
        NativeFunction::ReflectApply => reflect::reflect_apply,
        NativeFunction::ReflectConstruct => reflect::reflect_construct,
        NativeFunction::ReflectDefineProperty => reflect::reflect_define_property,
        NativeFunction::ReflectDeleteProperty => reflect::reflect_delete_property,
        NativeFunction::ReflectGet => reflect::reflect_get,
        NativeFunction::ReflectGetPrototypeOf => reflect::reflect_get_prototype_of,
        NativeFunction::ReflectGetOwnPropertyDescriptor => {
            reflect::reflect_get_own_property_descriptor
        }
        NativeFunction::ReflectHas => reflect::reflect_has,
        NativeFunction::ReflectIsExtensible => reflect::reflect_is_extensible,
        NativeFunction::ReflectOwnKeys => reflect::reflect_own_keys,
        NativeFunction::ReflectPreventExtensions => reflect::reflect_prevent_extensions,
        NativeFunction::ReflectSet => reflect::reflect_set,
        NativeFunction::ReflectSetPrototypeOf => reflect::reflect_set_prototype_of,
        NativeFunction::RegExp => regexp::reg_exp,
        NativeFunction::RegExpEscape => regexp::reg_exp_escape,
        NativeFunction::RegExpPrototypeCompile => regexp::reg_exp_prototype_compile,
        NativeFunction::RegExpPrototypeDotAll => regexp::reg_exp_prototype_dot_all,
        NativeFunction::RegExpPrototypeExec => regexp::reg_exp_prototype_exec,
        NativeFunction::RegExpPrototypeFlags => regexp::reg_exp_prototype_flags,
        NativeFunction::RegExpPrototypeGlobal => regexp::reg_exp_prototype_global,
        NativeFunction::RegExpPrototypeHasIndices => regexp::reg_exp_prototype_has_indices,
        NativeFunction::RegExpPrototypeIgnoreCase => regexp::reg_exp_prototype_ignore_case,
        NativeFunction::RegExpPrototypeMatch => regexp::reg_exp_prototype_match,
        NativeFunction::RegExpPrototypeMatchAll => regexp::reg_exp_prototype_match_all,
        NativeFunction::RegExpPrototypeMultiline => regexp::reg_exp_prototype_multiline,
        NativeFunction::RegExpPrototypeReplace => regexp::reg_exp_prototype_replace,
        NativeFunction::RegExpPrototypeSearch => regexp::reg_exp_prototype_search,
        NativeFunction::RegExpPrototypeSplit => regexp::reg_exp_prototype_split,
        NativeFunction::RegExpPrototypeSource => regexp::reg_exp_prototype_source,
        NativeFunction::RegExpPrototypeSticky => regexp::reg_exp_prototype_sticky,
        NativeFunction::RegExpPrototypeTest => regexp::reg_exp_prototype_test,
        NativeFunction::RegExpPrototypeToString => regexp::reg_exp_prototype_to_string,
        NativeFunction::RegExpPrototypeUnicode => regexp::reg_exp_prototype_unicode,
        NativeFunction::RegExpPrototypeUnicodeSets => regexp::reg_exp_prototype_unicode_sets,
        NativeFunction::RegExpStringIteratorPrototypeNext => {
            regexp::reg_exp_string_iterator_prototype_next
        }
        NativeFunction::String => strings::string,
        NativeFunction::StringFromCharCode => strings::string_from_char_code,
        NativeFunction::StringFromCodePoint => strings::string_from_code_point,
        NativeFunction::StringRaw => strings::string_raw,
        NativeFunction::StringPrototypeAt => strings::string_prototype_at,
        NativeFunction::StringPrototypeAnchor => strings::string_prototype_anchor,
        NativeFunction::StringPrototypeBig => strings::string_prototype_big,
        NativeFunction::StringPrototypeBlink => strings::string_prototype_blink,
        NativeFunction::StringPrototypeBold => strings::string_prototype_bold,
        NativeFunction::StringPrototypeCharAt => strings::string_prototype_char_at,
        NativeFunction::StringPrototypeCharCodeAt => strings::string_prototype_char_code_at,
        NativeFunction::StringPrototypeCodePointAt => strings::string_prototype_code_point_at,
        NativeFunction::StringPrototypeConcat => strings::string_prototype_concat,
        NativeFunction::StringPrototypeEndsWith => strings::string_prototype_ends_with,
        NativeFunction::StringPrototypeFixed => strings::string_prototype_fixed,
        NativeFunction::StringPrototypeFontcolor => strings::string_prototype_fontcolor,
        NativeFunction::StringPrototypeFontsize => strings::string_prototype_fontsize,
        NativeFunction::StringPrototypeIncludes => strings::string_prototype_includes,
        NativeFunction::StringPrototypeIndexOf => strings::string_prototype_index_of,
        NativeFunction::StringPrototypeItalics => strings::string_prototype_italics,
        NativeFunction::StringPrototypeIterator => strings::string_prototype_iterator,
        NativeFunction::StringPrototypeIsWellFormed => strings::string_prototype_is_well_formed,
        NativeFunction::StringIteratorPrototypeNext => strings::string_iterator_prototype_next,
        NativeFunction::StringPrototypeLastIndexOf => strings::string_prototype_last_index_of,
        NativeFunction::StringPrototypeLink => strings::string_prototype_link,
        NativeFunction::StringPrototypeLocaleCompare => strings::string_prototype_locale_compare,
        NativeFunction::StringPrototypeMatch => strings::string_prototype_match,
        NativeFunction::StringPrototypeMatchAll => strings::string_prototype_match_all,
        NativeFunction::StringPrototypeNormalize => strings::string_prototype_normalize,
        NativeFunction::StringPrototypePadEnd => strings::string_prototype_pad_end,
        NativeFunction::StringPrototypePadStart => strings::string_prototype_pad_start,
        NativeFunction::StringPrototypeRepeat => strings::string_prototype_repeat,
        NativeFunction::StringPrototypeReplace => strings::string_prototype_replace,
        NativeFunction::StringPrototypeReplaceAll => strings::string_prototype_replace_all,
        NativeFunction::StringPrototypeSearch => strings::string_prototype_search,
        NativeFunction::StringPrototypeSlice => strings::string_prototype_slice,
        NativeFunction::StringPrototypeSmall => strings::string_prototype_small,
        NativeFunction::StringPrototypeSplit => strings::string_prototype_split,
        NativeFunction::StringPrototypeStartsWith => strings::string_prototype_starts_with,
        NativeFunction::StringPrototypeStrike => strings::string_prototype_strike,
        NativeFunction::StringPrototypeSubstr => strings::string_prototype_substr,
        NativeFunction::StringPrototypeSubstring => strings::string_prototype_substring,
        NativeFunction::StringPrototypeSub => strings::string_prototype_sub,
        NativeFunction::StringPrototypeSup => strings::string_prototype_sup,
        NativeFunction::StringPrototypeToLowerCase => strings::string_prototype_to_lower_case,
        NativeFunction::StringPrototypeToLocaleLowerCase => {
            strings::string_prototype_to_locale_lower_case
        }
        NativeFunction::StringPrototypeToLocaleUpperCase => {
            strings::string_prototype_to_locale_upper_case
        }
        NativeFunction::StringPrototypeTrim => strings::string_prototype_trim,
        NativeFunction::StringPrototypeTrimEnd => strings::string_prototype_trim_end,
        NativeFunction::StringPrototypeTrimStart => strings::string_prototype_trim_start,
        NativeFunction::StringPrototypeToString => strings::string_prototype_to_string,
        NativeFunction::StringPrototypeToUpperCase => strings::string_prototype_to_upper_case,
        NativeFunction::StringPrototypeToWellFormed => strings::string_prototype_to_well_formed,
        NativeFunction::StringPrototypeValueOf => strings::string_prototype_value_of,
        NativeFunction::Symbol => core::symbol,
        NativeFunction::SymbolFor => core::symbol_for,
        NativeFunction::SymbolKeyFor => core::symbol_key_for,
        NativeFunction::SymbolPrototypeDescription => core::symbol_prototype_description,
        NativeFunction::SymbolPrototypeToPrimitive => core::symbol_prototype_to_primitive,
        NativeFunction::SymbolPrototypeToString => core::symbol_prototype_to_string,
        NativeFunction::SymbolPrototypeValueOf => core::symbol_prototype_value_of,
        NativeFunction::SyntaxError => errors::syntax_error,
        NativeFunction::TypeError => errors::type_error,
        NativeFunction::Unescape => core::unescape,
        NativeFunction::UriError => errors::uri_error,
    }
}

fn uninitialized_lexical(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    _args: &[Value],
) -> Result<Value, RuntimeError> {
    Err(RuntimeError {
        thrown: None,
        message: "ReferenceError: uninitialized lexical binding".to_owned(),
    })
}
