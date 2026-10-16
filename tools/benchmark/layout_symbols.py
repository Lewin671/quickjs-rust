"""Structural Rust v0 names for compiler-layout selection.

Decode the engine paths, type arguments and backreferences needed to distinguish
execution roles. Unsupported or malformed encodings return None; this is not a
complete general-purpose demangler. Canonical names help select a current raw
spelling, while linker audits must always compare the actual raw names.

Grammar reference: https://doc.rust-lang.org/rustc/symbol-mangling/v0.html
Reference-derived examples: rustc-demangle (MIT OR Apache-2.0); see
RUSTC_DEMANGLE_LICENSE.md for its notice.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, Mapping, Sequence

# The nesting depth after which a symbol is called invalid rather than parsed,
# matching the reference implementation's own recursion limit.
MAX_DEPTH = 500

# Namespace tag -> the label a special (non-lowercase) namespace prints.
SPECIAL_NAMESPACES = {"C": "closure", "S": "shim"}

# A `v0` name may carry one vendor-specific suffix, and the grammar allows it
# to start only with one of these (see the "suffix" rule of the v0 spec).
SUFFIX_STARTS = (".", "$")

BASIC_TYPES = {
    "b": "bool", "c": "char", "e": "str", "u": "()", "a": "i8", "s": "i16",
    "l": "i32", "x": "i64", "n": "i128", "i": "isize", "h": "u8", "t": "u16",
    "m": "u32", "y": "u64", "o": "u128", "j": "usize", "f": "f32", "d": "f64",
    "z": "!", "p": "_", "v": "...",
}


class SymbolSyntaxError(ValueError):
    """A `v0` symbol that violates the grammar, with the byte offset."""

    def __init__(self, message: str, offset: int | None = None) -> None:
        super().__init__(message if offset is None else f"{message} at byte {offset}")
        self.offset = offset


@dataclass(frozen=True)
class Crate:
    name: str


@dataclass(frozen=True)
class Nested:
    namespace: str
    parent: "Path"
    disambiguator: int
    name: str


@dataclass(frozen=True)
class Generic:
    base: "Path"
    args: tuple["Node", ...]


@dataclass(frozen=True)
class Impl:
    tag: str
    self_type: "Path"
    trait: "Path | None"


@dataclass(frozen=True)
class Type:
    tag: str
    parts: tuple["Node", ...]


@dataclass(frozen=True)
class RawType:
    """A basic type such as `u8`, named by its tag and its encoded text."""

    tag: str
    text: str


@dataclass(frozen=True)
class Marker:
    """A detail of a type that carries no tree of its own: `unsafe`, an ABI,
    or a pattern. It keeps two otherwise identical canonical names apart."""

    text: str


@dataclass(frozen=True)
class Lifetime:
    index: int


@dataclass(frozen=True)
class Const:
    text: str


Path = Crate | Nested | Generic | Impl
Node = Path | Type | RawType | Marker | Lifetime | Const


@dataclass(frozen=True)
class Symbol:
    """A parsed `v0` symbol: its spelling, path tree and match identity.

    `path` is the decoded tree as written; `canonical` is what matching uses,
    so the wrappers of unnamed closures around the path are already removed
    and `wrapped` records whether there were any.
    """

    text: str
    path: Path
    suffix: str
    canonical: str

    @property
    def wrapped(self) -> bool:
        return unwrap_closures(self.path) is not self.path


def unwrap_closures(path: Node) -> Node:
    """`path` without the wrappers of unnamed closures around it."""
    while isinstance(path, Nested) and path.namespace == "C" and not path.name:
        path = path.parent
    return path


def canonical_name(node: Node) -> str:
    """The match identity of a path: see the module docstring."""
    if isinstance(node, Crate):
        return node.name
    if isinstance(node, Nested):
        base = canonical_name(node.parent)
        if node.namespace.isupper():
            label = SPECIAL_NAMESPACES.get(node.namespace, node.namespace)
            return f"{base}::{{{label}:{node.name}}}" if node.name else f"{base}::{{{label}}}"
        return f"{base}::{node.name}" if node.name else base
    if isinstance(node, Generic):
        return f"{canonical_name(node.base)}::<{', '.join(map(canonical_name, node.args))}>"
    if isinstance(node, Impl):
        text = f"<{canonical_name(node.self_type)}"
        return text + f" as {canonical_name(node.trait)}>" if node.trait else text + ">"
    if isinstance(node, Type):
        return f"{node.tag}<{', '.join(map(canonical_name, node.parts))}>"
    if isinstance(node, RawType):
        return BASIC_TYPES.get(node.tag, f"{{{node.tag}}}")
    if isinstance(node, Marker):
        return f"{{{node.text}}}"
    if isinstance(node, Lifetime):
        return "'_" if node.index == 0 else f"'_{node.index}"
    if isinstance(node, Const):
        return node.text
    raise TypeError(f"not a v0 node: {node!r}")


def crate_names(node: Node) -> tuple[str, ...]:
    """Every crate root named anywhere in `node`, once each, in tree order."""
    found: list[str] = []

    def walk(current: Node) -> None:
        if isinstance(current, Crate):
            if current.name not in found:
                found.append(current.name)
        elif isinstance(current, Nested):
            walk(current.parent)
        elif isinstance(current, Generic):
            walk(current.base)
            for arg in current.args:
                walk(arg)
        elif isinstance(current, Impl):
            walk(current.self_type)
            if current.trait is not None:
                walk(current.trait)
        elif isinstance(current, Type):
            for part in current.parts:
                walk(part)

    walk(node)
    return tuple(found)


class _Parser:
    def __init__(self, symbol: str, position: int = 0, depth: list[int] | None = None) -> None:
        self.symbol = symbol
        self.position = position
        self.depth = depth if depth is not None else [0]

    # -- cursor ------------------------------------------------------
    def peek(self) -> str | None:
        return self.symbol[self.position] if self.position < len(self.symbol) else None

    def eat(self, char: str) -> bool:
        if self.peek() == char:
            self.position += 1
            return True
        return False

    def take(self) -> str:
        char = self.peek()
        if char is None:
            raise SymbolSyntaxError("unexpected end of symbol", self.position)
        self.position += 1
        return char

    def enter(self) -> None:
        self.depth[0] += 1
        if self.depth[0] > MAX_DEPTH:
            raise SymbolSyntaxError("recursion limit reached", self.position)

    def leave(self) -> None:
        self.depth[0] -= 1

    def decimal(self) -> int:
        """One decimal digit, the start of an identifier's length."""
        char = self.peek()
        if char is None or not "0" <= char <= "9":
            raise SymbolSyntaxError("expected a base-10 digit", self.position)
        self.position += 1
        return ord(char) - 48

    def base62(self) -> int:
        """Base-62 digits terminated by `_`, plus one (the `v0` encoding)."""
        if self.eat("_"):
            return 0
        value = 0
        seen = False
        while (char := self.peek()) is not None:
            if "0" <= char <= "9":
                digit = ord(char) - 48
            elif "a" <= char <= "z":
                digit = 10 + ord(char) - 97
            elif "A" <= char <= "Z":
                digit = 36 + ord(char) - 65
            else:
                break
            self.position += 1
            value = value * 62 + digit
            seen = True
        if not seen or not self.eat("_"):
            raise SymbolSyntaxError("unterminated base-62 integer", self.position)
        return value + 1

    def optional_s(self) -> int:
        """A `s<base-62>_` disambiguator, or 0 when absent."""
        if not self.eat("s"):
            return 0
        return self.base62() + 1

    # -- grammar -----------------------------------------------------
    def identifier(self) -> tuple[int, str]:
        punycode = self.eat("u")
        length = self.decimal()
        if length != 0:
            while (char := self.peek()) is not None and "0" <= char <= "9":
                self.position += 1
                length = length * 10 + ord(char) - 48
        self.eat("_")
        end = self.position + length
        if end > len(self.symbol):
            raise SymbolSyntaxError("identifier runs past the symbol", self.position)
        name = self.symbol[self.position:end]
        self.position = end
        if punycode and not name.rpartition("_")[2]:
            raise SymbolSyntaxError("empty punycode identifier", self.position)
        return punycode, name

    def backreference(self, parse):
        """Re-parse a backreference target in the caller's grammar context."""
        start = self.position - 1
        index = self.base62()
        if index >= start:
            raise SymbolSyntaxError("backreference does not point backwards", start)
        target = _Parser(self.symbol, index, self.depth)
        target.enter()
        try:
            return parse(target)
        finally:
            target.leave()

    def path(self, in_value: bool) -> Path:
        self.enter()
        try:
            tag = self.take()
            if tag == "C":
                self.optional_s()
                return Crate(self.identifier()[1])
            if tag == "N":
                namespace = self.take()
                if not (namespace.isascii() and namespace.isalpha()):
                    raise SymbolSyntaxError("bad namespace tag", self.position)
                parent = self.path(in_value)
                return Nested(namespace, parent, self.optional_s(), self.identifier()[1])
            if tag in {"M", "X", "Y"}:
                if tag != "Y":
                    self.optional_s()
                    self.path(False)
                self_type = self.type()
                trait = self.path(False) if tag != "M" else None
                return Impl(tag, self_type, trait)
            if tag == "I":
                base = self.path(in_value)
                args: list[Node] = []
                while not self.eat("E"):
                    args.append(self.generic_arg())
                return Generic(base, tuple(args))
            if tag == "B":
                return self.backreference(lambda target: target.path(in_value))
            raise SymbolSyntaxError(f"unsupported path tag {tag!r}", self.position - 1)
        finally:
            self.leave()

    def generic_arg(self) -> Node:
        if self.eat("L"):
            return Lifetime(self.base62())
        if self.eat("K"):
            return Const(self.constant_text())
        return self.type()

    def constant_text(self) -> str:
        start = self.position
        self.constant()
        return self.symbol[start:self.position]

    def constant(self) -> None:
        self.enter()
        try:
            tag = self.take()
            if tag == "B":
                self.backreference(lambda target: target.constant())
            elif tag in {"h", "t", "m", "y", "o", "j"}:
                self.hex_nibbles()
            elif tag == "p":
                pass
            elif tag in {"a", "s", "l", "x", "n", "i"}:
                self.eat("n")
                self.hex_nibbles()
            elif tag in {"b", "c", "e"}:
                self.hex_nibbles()
            elif tag in {"R", "Q"}:
                if tag == "R" and self.eat("e"):
                    self.hex_nibbles()
                else:
                    self.constant()
            elif tag in {"A", "T"}:
                while not self.eat("E"):
                    self.constant()
            elif tag == "V":
                self.path(True)
                kind = self.take()
                if kind == "T":
                    while not self.eat("E"):
                        self.constant()
                elif kind == "S":
                    while not self.eat("E"):
                        self.optional_s()
                        self.identifier()
                        self.constant()
                elif kind != "U":
                    raise SymbolSyntaxError(f"bad enum variant tag {kind!r}", self.position - 1)
            else:
                raise SymbolSyntaxError(f"unsupported const tag {tag!r}", self.position - 1)
        finally:
            self.leave()

    def hex_nibbles(self) -> None:
        seen = False
        while (char := self.peek()) is not None:
            if char == "_":
                self.position += 1
                if not seen:
                    raise SymbolSyntaxError("empty constant", self.position)
                return
            if not ("0" <= char <= "9" or "a" <= char <= "f"):
                break
            self.position += 1
            seen = True
        raise SymbolSyntaxError("unterminated constant", self.position)

    def type(self) -> Node:
        self.enter()
        try:
            return self._type()
        finally:
            self.leave()

    def _type(self) -> Node:
        start = self.position
        tag = self.peek()
        if tag is None:
            raise SymbolSyntaxError("unexpected end of symbol", start)
        if tag in BASIC_TYPES:
            self.position += 1
            return RawType(tag, self.symbol[start:self.position])
        if tag in {"R", "Q"}:
            self.position += 1
            if self.eat("L"):
                self.base62()
            return Type(tag, (self.type(),))
        if tag in {"P", "O"}:
            self.position += 1
            return Type(tag, (self.type(),))
        if tag in {"A", "S"}:
            self.position += 1
            inner = self.type()
            parts = (inner, Const(self.constant_text())) if tag == "A" else (inner,)
            return Type(tag, parts)
        if tag == "T":
            self.position += 1
            parts = []
            while not self.eat("E"):
                parts.append(self.type())
            return Type(tag, tuple(parts))
        if tag == "F":
            self.position += 1
            if self.eat("G"):
                self.base62()
            unsafe = self.eat("U")
            abi = ""
            if self.eat("K"):
                abi = "C" if self.eat("C") else self.identifier()[1]
            args = []
            while not self.eat("E"):
                args.append(self.type())
            returns = None if self.eat("u") else self.type()
            markers = (Marker("unsafe" if unsafe else "safe"), Marker(f"abi {abi or 'Rust'}"))
            return Type(tag, markers + tuple(args)
                        + (() if returns is None else (returns,)))
        if tag == "D":
            self.position += 1
            if self.eat("G"):
                self.base62()
            parts: list[Node] = []
            while not self.eat("E"):
                parts.append(self.dyn_trait())
            if not self.eat("L"):
                raise SymbolSyntaxError("trait object without lifetime", self.position)
            parts.append(Lifetime(self.base62()))
            return Type(tag, tuple(parts))
        if tag == "W":
            self.position += 1
            inner = self.type()
            return Type(tag, (inner, Marker(self.pattern_text())))
        if tag == "B":
            self.position += 1
            return self.backreference(lambda target: target.type())
        return self.path(False)

    def dyn_trait(self) -> Node:
        """One trait of a `dyn` object: its path and any associated bindings."""
        if self.eat("B"):
            return self.backreference(lambda target: target.dyn_trait())
        if self.eat("I"):
            base = self.path(False)
            args: list[Node] = []
            while not self.eat("E"):
                args.append(self.generic_arg())
            trait: Node = Generic(base, tuple(args))
        else:
            trait = self.path(False)
        bindings: list[Node] = []
        while self.eat("p"):
            name = self.identifier()[1]
            value: Node = Const(self.constant_text()) if self.eat("K") else self.type()
            bindings.append(Type("binding", (Marker(name), value)))
        return Type("dyn", (trait, *bindings))

    def pattern_text(self) -> str:
        start = self.position
        self.pattern()
        return self.symbol[start:self.position]

    def pattern(self) -> None:
        self.enter()
        try:
            tag = self.take()
            if tag == "R":
                self.constant()
                self.constant()
            elif tag == "O":
                self.pattern()
                while not self.eat("E"):
                    self.pattern()
            elif tag != "N":
                raise SymbolSyntaxError(f"unsupported pattern tag {tag!r}", self.position - 1)
        finally:
            self.leave()


def _mangling_prefix(text: str) -> tuple[str, int] | None:
    """(inner symbol, its length) for a `v0` name, or None for anything else."""
    if text.startswith("__R"):
        inner = text[3:]
    elif text.startswith("_R"):
        inner = text[2:]
    else:
        return None
    if not inner or not "A" <= inner[0] <= "Z":
        return None
    if any(ord(char) & 0x80 for char in inner):
        return None
    return inner, len(inner)


def parse_symbol(text: str) -> Symbol | None:
    """Decode one `v0` symbol, or return `None` if it is not a complete one.

    Only `_R`/`__R`-prefixed names whose first path tag is uppercase are
    considered, as in `rustc-demangle`: an undefined, C or Swift name is not a
    Rust symbol and never becomes a match. After the path and its optional
    instantiating crate the grammar allows one vendor-specific suffix, and it
    must start with `.` or `$`; any other trailing text is rejected rather
    than passed through as a suffix.
    """
    prefix = _mangling_prefix(text)
    if prefix is None:
        return None
    inner, length = prefix
    parser = _Parser(inner)
    try:
        path = parser.path(True)
        if (char := parser.peek()) is not None and "A" <= char <= "Z":
            parser.path(True)
        suffix = inner[parser.position:]
        if suffix and suffix[0] not in SUFFIX_STARTS:
            return None
    except SymbolSyntaxError:
        return None
    except RecursionError:
        # A symbol nested more deeply than the interpreter can descend is not
        # a symbol this decoder can accept; say so instead of crashing.
        return None
    return Symbol(text, path, suffix, canonical_name(unwrap_closures(path)))


@dataclass(frozen=True)
class SymbolEntry:
    """One spelling of a canonical function, and its vendor-specific suffix."""

    text: str
    symbol: Symbol

    @property
    def suffix(self) -> str:
        return self.symbol.suffix

    @property
    def canonical(self) -> str:
        return self.symbol.canonical


class SymbolIndex:
    """The canonical names of a set of symbols, with every spelling of each.

    Matching goes through this index so one parse per symbol is enough, and so
    that a target matched by several spellings stays visible instead of being
    reduced to the first one. No spelling is preferred here: which body a
    name refers to is decided by address, in `layout_pin.definitions`, not by
    how the linker happened to spell it.
    """

    def __init__(self, names: Iterable[str]) -> None:
        self._entries: dict[str, list[SymbolEntry]] = {}
        self._canonical: dict[str, str] = {}
        self.unparsable: list[str] = []
        for name in names:
            if name in self._canonical:
                continue
            symbol = parse_symbol(name)
            if symbol is None:
                self.unparsable.append(name)
                continue
            self._canonical[name] = symbol.canonical
            self._entries.setdefault(symbol.canonical, []).append(SymbolEntry(name, symbol))
        for entries in self._entries.values():
            entries.sort(key=lambda entry: entry.text)

    def __contains__(self, canonical: str) -> bool:
        return canonical in self._entries

    def __len__(self) -> int:
        return len(self._canonical)

    @property
    def canonicals(self) -> Mapping[str, Sequence[SymbolEntry]]:
        return self._entries

    def canonical_of(self, text: str) -> str | None:
        return self._canonical.get(text)

    def match(self, canonical: str) -> tuple[SymbolEntry, ...]:
        """Every spelling in this set whose canonical name is `canonical`."""
        return tuple(self._entries.get(canonical, ()))
