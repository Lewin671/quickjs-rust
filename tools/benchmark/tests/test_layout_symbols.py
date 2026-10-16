"""Decoding tests for `layout_symbols`.

The `v0` examples marked "reference corpus" are the ones the reference
implementation, `rustc-demangle` (MIT OR Apache-2.0), uses in its own test
suite; they are reproduced here to pin the grammar this decoder accepts. The
engine symbols come from this repository's recorded `nm` and order file, so the
shapes that actually occur are covered by real spellings.
"""

from __future__ import annotations

import unittest

from tools.benchmark import layout_symbols as symbols
from tools.benchmark.tests.layout_fixtures import (
    CALL_LEAF, CALL_LEAF_OLD, CLONE, DROP_IN_PLACE, INTERPRETER_CLOSURE, INTERPRETER_FALLBACK,
    INTERPRETER_PLAIN_OLD, WIDE_CLOSURE, WIDE_PLAIN_OLD,
)

WIDE = ("qjs_runtime::bytecode::typed_loop::execute::run::"
        "<qjs_runtime::bytecode::compact_fn::wide::loop_frame::WideLoopFrame>")
INTERPRETER = "qjs_runtime::bytecode::typed_loop::execute::run::<qjs_runtime::bytecode::vm::Vm>"


def canonical(text: str) -> str:
    symbol = symbols.parse_symbol(text)
    assert symbol is not None, text
    return symbol.canonical


class CanonicalNameTests(unittest.TestCase):
    def test_crate_disambiguator_backrefs_and_closure_wrapper_do_not_matter(self):
        # The same loop, spelled by two builds and wrapped in a closure by the
        # newer compiler, is one canonical name.
        self.assertEqual(canonical(WIDE_CLOSURE), WIDE)
        self.assertEqual(canonical(WIDE_PLAIN_OLD), WIDE)
        self.assertEqual(canonical(INTERPRETER_CLOSURE), INTERPRETER)
        self.assertEqual(canonical(INTERPRETER_PLAIN_OLD), INTERPRETER)
        self.assertEqual(canonical(CALL_LEAF), canonical(CALL_LEAF_OLD))

    def test_the_two_executor_instantiations_stay_distinct(self):
        # `Vm` and `WideLoopFrame` differ in one generic argument and nothing
        # else, so a matcher that ignored the argument would conflate them.
        self.assertNotEqual(WIDE, INTERPRETER)
        vm_fallback = canonical(INTERPRETER_FALLBACK)
        self.assertIn("try_run_typed_loop::<qjs_runtime::bytecode::vm::Vm>", vm_fallback)
        self.assertNotIn("WideLoopFrame", vm_fallback)

    def test_wrapper_is_reported_and_removed(self):
        wrapped, plain = symbols.parse_symbol(WIDE_CLOSURE), symbols.parse_symbol(WIDE_PLAIN_OLD)
        self.assertTrue(wrapped.wrapped)
        self.assertFalse(plain.wrapped)
        self.assertIn("{closure}", symbols.canonical_name(wrapped.path))

    def test_trait_impl_and_vendor_suffix(self):
        # The linker's duplicate suffix is not part of the identity, so both
        # spellings of one callee share a canonical name.
        self.assertEqual(canonical(DROP_IN_PLACE + ".3946"), canonical(DROP_IN_PLACE))
        self.assertEqual(symbols.parse_symbol(DROP_IN_PLACE + ".3946").suffix, ".3946")
        self.assertEqual(symbols.parse_symbol(DROP_IN_PLACE).suffix, "")
        self.assertIn("<qjs_runtime::value::Value as core::clone::Clone>::clone", canonical(CLONE))

    def test_reference_corpus_paths_and_closures(self):
        self.assertEqual(symbols.canonical_name(
            symbols.parse_symbol("_RNvC6_123foo3bar").path), "123foo::bar")
        symbol = symbols.parse_symbol("_RNCNCNgCs6DXkGYLi8lr_2cc5spawn00B5_")
        self.assertEqual(symbols.canonical_name(symbol.path), "cc::spawn::{closure}::{closure}")
        self.assertEqual(symbols.canonical_name(symbols.unwrap_closures(symbol.path)),
                         "cc::spawn")

    def test_reference_corpus_dyn_trait_keeps_its_parts(self):
        # A `dyn` type is structure, not a placeholder: the trait path and the
        # associated binding both stay in the canonical name.
        text = ("_RINbNbCskIICzLVDPPb_5alloc5alloc8box_freeDINbNiB4_5boxed5FnBoxuEp6OutputuEL_"
                "ECs1iopQbuBiw2_3std")
        rendered = canonical(text)
        self.assertIn("box_free", rendered)
        self.assertIn("alloc::boxed::FnBox", rendered)
        self.assertIn("{Output}", rendered)

    def test_reference_corpus_const_argument_and_thin_lto_suffix(self):
        # A const generic argument keeps its encoded text; the encodings of
        # literal constants are self-contained, so they are stable across
        # builds even though they are not decoded into a value here.
        self.assertEqual(canonical("_RIC0Khb_E"), "::<hb_>")
        suffix = symbols.parse_symbol("_RNvC9backtrace3foo.llvm.A5310EB9")
        self.assertEqual(suffix.suffix, ".llvm.A5310EB9")
        self.assertEqual(symbols.canonical_name(suffix.path), "backtrace::foo")

    def test_reference_corpus_punycode_identifier_parses(self):
        text = "_RNqCs4fqI2P2rA04_11utf8_identsu30____7hkackfecea1cbdathfdh9hlq6y"
        symbol = symbols.parse_symbol(text)
        self.assertIsNotNone(symbol)
        self.assertEqual(symbol.suffix, "")
        self.assertTrue(symbol.canonical.startswith("utf8_idents::"), symbol.canonical)

    def test_markers_keep_two_types_apart(self):
        # A marker is the part of a type with no tree of its own; it still has
        # to distinguish the two names it appears in.
        unsafe = symbols.Type("F", (symbols.Marker("unsafe"),))
        safe = symbols.Type("F", (symbols.Marker("safe"),))
        self.assertNotEqual(symbols.canonical_name(unsafe), symbols.canonical_name(safe))


class RejectionTests(unittest.TestCase):
    def test_names_that_are_not_v0_rust_symbols(self):
        for text in ("_main", "_rust_eh_personality", "___floattidf", "__mh_execute_header",
                     "_Z3foov", "_R", "_RC99std", "foo", ""):
            with self.subTest(text=text):
                self.assertIsNone(symbols.parse_symbol(text))

    def test_truncated_symbols(self):
        for text in ("__RNvNtNtNtCs93CNxA9ZQ5c_11qjs_runtime8bytecode10typed_loop7execute",
                     "__RNC", "__RNv", "__RINtC4crate3foo",
                     # an identifier length that runs past the symbol
                     "__RNvC4crate5foo"):
            with self.subTest(text=text):
                self.assertIsNone(symbols.parse_symbol(text))

    def test_trailing_text_must_be_a_vendor_specific_suffix(self):
        # The grammar allows exactly one suffix after the instantiating crate,
        # and it starts with `.` or `$`; anything else is not a symbol.
        self.assertIsNone(symbols.parse_symbol("_RNvC5crate3fooJUNK"))
        self.assertIsNone(symbols.parse_symbol("_RNvC5crate3foo JUNK"))
        self.assertEqual(symbols.parse_symbol("_RNvC5crate3foo$suffix").suffix, "$suffix")
        self.assertEqual(symbols.parse_symbol("_RNvC5crate3foo.llvm.1").suffix, ".llvm.1")

    def test_deep_nesting_is_rejected_rather_than_crashed(self):
        shallow = symbols.parse_symbol("_RIC0" + "R" * 100 + "p" + "E")
        self.assertIsNotNone(shallow)
        deep = symbols.parse_symbol("_RIC0" + "R" * (symbols.MAX_DEPTH * 2) + "p" + "E")
        self.assertIsNone(deep)


class SymbolIndexTests(unittest.TestCase):
    def test_every_spelling_of_a_canonical_function_is_kept(self):
        index = symbols.SymbolIndex([WIDE_CLOSURE, WIDE_PLAIN_OLD, WIDE_CLOSURE, "_main"])
        self.assertEqual(len(index), 2)
        self.assertEqual({entry.text for entry in index.match(WIDE)},
                         {WIDE_CLOSURE, WIDE_PLAIN_OLD})
        self.assertEqual(index.match(INTERPRETER), ())
        self.assertEqual(index.unparsable, ["_main"])

    def test_canonical_of_round_trips(self):
        index = symbols.SymbolIndex([CALL_LEAF])
        self.assertEqual(index.canonical_of(CALL_LEAF),
                         "qjs_runtime::bytecode::typed_loop::execute::call_leaf")
        self.assertIsNone(index.canonical_of("_main"))

    def test_crate_names_finds_every_crate_in_a_path(self):
        parsed = symbols.parse_symbol(WIDE_CLOSURE)
        self.assertEqual(symbols.crate_names(parsed.path), ("qjs_runtime",))
        # The instantiating crate is part of the spelling, not of the path.
        thin = symbols.parse_symbol("_RNvC9backtrace3foo.llvm.A5310EB9")
        self.assertEqual(symbols.crate_names(thin.path), ("backtrace",))


if __name__ == "__main__":
    unittest.main()
