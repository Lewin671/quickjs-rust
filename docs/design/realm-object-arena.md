# Realm-local object arena

## Status

Rejected by T023 S1 on 2026-07-30. The implementation was removed after its
three-block external screen regressed the HashMap target to 1.016x baseline
(required <=0.950x) and A* to 1.088x (required <=1.030x). The retained report
is `target/performance-realm-object-arena-fast-external/external-report.json`
(SHA-256 `2123ea9e6b9fd0020c04e4f38349046a3368d928c7b2ccd3f280ac2c127d0843`).

This document records a closed design, not a pending implementation. Do not
retry intrusive object cells, a Realm-local fixed-block arena, alternate
packed-count encodings, or wider arena admission under T023. Any future
tracing/GC project needs a fresh profile and a new T022 performance unit.

## Problem

`ObjectRef` currently wraps `Rc<ObjectData>`. That gives correct stable
identity and weak cache handles, but every ordinary receiver enters the system
allocator as a separate `Rc` allocation. Current rank-two/rank-three external
profiles show tens of thousands of those allocations in one Realm even when
all objects remain live together, so a reuse cache cannot solve the primary
cost.

## S1 representation

An object handle remains pointer-sized. Its cell stores packed strong/weak
counts and `ObjectData`; a Realm-local arena owns stable fixed-size blocks of
cells. The public object APIs retain their current ownership contract:

- cloning an `ObjectRef` increments the strong count;
- downgrading creates a non-owning weak handle;
- the final strong drop destroys `ObjectData` but leaves the cell available to
  weak handles;
- the final weak drop makes the slot reusable;
- arena ownership outlives its Realm owner while any live strong or weak cell
  remains.

This is an allocation strategy, not a moving collector. Reference cycles have
the same retention behavior as the current `Rc` model. A later collector must
add explicit roots and tracing; it must not be smuggled into S1.

## Safety boundary

The block allocator needs raw stable pointers, so unsafe code is restricted to
one internal module. It may expose only safe strong/weak operations to
`ObjectRef`. Slot reuse is permitted only after the strong and weak counts
both reach zero. The module documents the validity proof beside every unsafe
operation and has focused tests for clone/drop, weak upgrade, final drop,
reuse, and Realm-owner release.

## Why this is general

Arena admission depends only on the allocation domain and object kind. It
never examines JavaScript source, function/constructor names, property names,
benchmark paths, inputs, iteration counts, or results. Existing fallback
allocations use the same handle semantics, allowing migration one allocation
family at a time without creating a second object model.
