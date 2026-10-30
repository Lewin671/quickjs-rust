"""Build diagnostic-only source copies; never patch the timed checkout.

Usage: python3 ... <git revision> <output directory>
Counts actual storage operations, schema-owner Rc activity, atom/lease activity,
allocator requests and storage RefCell borrows. Both variants use the same probe.
"""
from pathlib import Path
import re
import subprocess
import sys

revision, destination = sys.argv[1:]
out = Path(destination).resolve()
out.mkdir(parents=True, exist_ok=False)
p = subprocess.Popen(['git', 'archive', revision], stdout=subprocess.PIPE)
subprocess.run(['tar', '-x', '-C', str(out)], stdin=p.stdout, check=True)
p.stdout.close()
assert p.wait() == 0
src = out / 'crates/qjs-runtime/src'
fields = '''r2_objects_created r2_arrays_created r2_store_adds r2_slot_allocs
r2_slot_reallocs r2_slot_capacity_bytes r2_slot_adopted_buffers
r2_schema_acquires r2_schema_clones r2_schema_drops r2_schema_upgrades
r2_lease_clones r2_lease_drops r2_lease_upgrades r2_ref_reads r2_ref_writes
r2_hashes r2_store_ic_probes r2_inline_pairs'''.split()
diag = src / 'diagnostics.rs'
s = diag.read_text().replace('declare_counters! {', 'declare_counters! {\n'+''.join('    '+f+',\n' for f in fields), 1)
s += '''
/// Diagnostic-only schema owners; no payload/ownership semantics change.
pub(crate) mod construction {
    use std::{cell::{Ref, RefCell, RefMut}, ops::{Deref, DerefMut}, rc::Rc};
    pub(crate) struct SchemaRc<T>(Rc<T>);
    impl<T> From<Rc<T>> for SchemaRc<T> {
        fn from(value: Rc<T>) -> Self { super::count!(r2_schema_acquires); Self(value) }
    }
    impl<T> From<T> for SchemaRc<T> {
        fn from(value: T) -> Self { Rc::new(value).into() }
    }
    impl<T> SchemaRc<T> {
        pub(crate) fn from_weak(value: Rc<T>) -> Self {
            super::count!(r2_schema_upgrades); value.into()
        }
        pub(crate) fn clone_raw(value: &Rc<T>) -> Self {
            super::count!(r2_schema_clones); value.clone().into()
        }
    }
    impl<T> Clone for SchemaRc<T> {
        fn clone(&self) -> Self { Self::clone_raw(&self.0) }
    }
    impl<T> Drop for SchemaRc<T> {
        fn drop(&mut self) { super::count!(r2_schema_drops); }
    }
    impl<T> Deref for SchemaRc<T> { type Target = Rc<T>; fn deref(&self) -> &Rc<T> { &self.0 } }
    impl<T> DerefMut for SchemaRc<T> { fn deref_mut(&mut self) -> &mut Rc<T> { &mut self.0 } }
    impl<T> std::fmt::Debug for SchemaRc<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("SchemaRc") }
    }
    pub(crate) fn read<T>(cell: &RefCell<T>) -> Ref<'_, T> {
        super::count!(r2_ref_reads); cell.borrow()
    }
    pub(crate) fn write<T>(cell: &RefCell<T>) -> RefMut<'_, T> {
        super::count!(r2_ref_writes); cell.borrow_mut()
    }
    pub(crate) fn capacity<T>(before: usize, after: usize) {
        if after > before {
            if before==0 { super::count!(r2_slot_allocs); } else { super::count!(r2_slot_reallocs); }
            super::update(|c| c.r2_slot_capacity_bytes += ((after-before)*std::mem::size_of::<T>()) as u64);
        }
    }
    pub(crate) fn push<T>(values: &mut Vec<T>, value: T) {
        let before=values.capacity(); values.push(value); capacity::<T>(before,values.capacity());
    }
    pub(crate) fn reserve<T>(values: &mut Vec<T>, additional: usize) {
        let before=values.capacity(); values.reserve(additional); capacity::<T>(before,values.capacity());
    }
}
'''
diag.write_text(s)
C = 'crate::diagnostics::construction'

def edit(file, transform):
    p = src / file
    p.write_text(transform(p.read_text()))

def count_body(s, signature, field):
    start=s.index(signature); brace=s.index('{',start)
    return s[:brace+1]+'\n    crate::diagnostics::count!('+field+');'+s[brace+1:]

def wrap_rc(s, prefix):
    """Wrap balanced Rc::new expressions following the requested prefix."""
    cursor=0
    while True:
        a=s.find(prefix,cursor)
        if a<0:break
        a=s.index('Rc::new(',a)
        b=a+len('Rc::new(');depth=1
        while depth:
            depth+=(s[b]=='(')-(s[b]==')');b+=1
        s=s[:b]+'.into()'+s[b:];cursor=b+7
    return s

candidate=(src/'value/layout').exists()
# Both revisions: count ordinary headers and array allocations at the actual
# allocation, not at clones of their public handles.
edit('value/object.rs',lambda s:s.replace('Self(Rc::new(ObjectData {','{ crate::diagnostics::count!(r2_objects_created); Self(Rc::new(ObjectData {').replace('            cold: OnceCell::new(),\n        }))','            cold: OnceCell::new(),\n        })) }'))
# The header initializer differs between revisions; locate each allocation and
# insert a count directly before the expression, keeping its surrounding block.
p=src/'value/object.rs';s=p.read_text()
# Undo the above balanced-block shortcut and use a small expression block.
s=s.replace('{ crate::diagnostics::count!(r2_objects_created); Self(Rc::new(ObjectData {','Self(Rc::new(ObjectData {').replace('        })) }','        }))')
indices=[m.start() for m in re.finditer(r'Rc::new\(ObjectData \{',s)]
for a in reversed(indices):
 b=a+len('Rc::new(');d=1
 while d:d+=(s[b]=='(')-(s[b]==')');b+=1
 s=s[:a]+'{ crate::diagnostics::count!(r2_objects_created); '+s[a:b]+' }'+s[b:]
p.write_text(s)
p=src/'value/array.rs';s=p.read_text();s=s.replace('Self(Rc::new(data))','{ crate::diagnostics::count!(r2_arrays_created); Self(Rc::new(data)) }')
indices=[m.start() for m in re.finditer(r'Rc::new\(ArrayData \{',s)]
for a in reversed(indices):
 b=a+len('Rc::new(');d=1
 while d:d+=(s[b]=='(')-(s[b]==')');b+=1
 s=s[:a]+'{ crate::diagnostics::count!(r2_arrays_created); '+s[a:b]+' }'+s[b:]
p.write_text(s)
# The base ordered builder allocates its header in a child module.
for p in (src/'value/object').glob('*.rs'):
    if p.name=='tests.rs': continue
    s=p.read_text();indices=[m.start() for m in re.finditer(r'Rc::new\(ObjectData \{',s)]
    for a in reversed(indices):
        b=a+len('Rc::new(');depth=1
        while depth:depth+=(s[b]=='(')-(s[b]==')');b+=1
        s=s[:a]+'{ crate::diagnostics::count!(r2_objects_created); '+s[a:b]+' }'+s[b:]
    p.write_text(s)
# Hash completion counts a table hash once, including resize rehashing.
edit('value/name_hash.rs',lambda s:count_body(s,'fn finish(&self)', 'r2_hashes'))
# Capture actual object/schema/interner storage borrows, excluding CallEnv/TLS.
for p in src.rglob('*.rs'):
    if p.name.endswith('tests.rs') or '/tests/' in str(p):continue
    if 'value/object' not in str(p) and 'value/layout' not in str(p) and p != src/'property/atom.rs':continue
    s=p.read_text()
    pattern=r'(?<![\w.])((?:self|object|current|prototype|data|layout|owner|child)(?:\s*\.\s*\w+)*\s*\.(?:properties|data|transitions|table))\s*\.borrow(_mut)?\(\)'
    s=re.sub(pattern,lambda m:C+('::write(&' if m[2] else '::read(&')+re.sub(r'\s+','',m[1])+')',s)
    p.write_text(s)
if candidate:
    for p in src.rglob('*.rs'):
        s=p.read_text()
        # Borrowed Rc parameters retain the standard type and coerce through
        # Deref; owned references are tracked at every clone/drop boundary.
        s=re.sub(r'(?<!&)Rc<((?:crate::value::layout::)?Layout)>', lambda m:C+'::SchemaRc<'+m[1]+'>',s)
        p.write_text(s)
    edit('value/layout/mod.rs',lambda s:wrap_rc(s,'root: Rc::new(Layout {'))
    edit('value/layout/edit.rs',lambda s:s.replace(') -> Rc<Self> {',') -> '+C+'::SchemaRc<Self> {').replace('Some(self.clone())','Some('+C+'::SchemaRc::clone_raw(self))').replace('Some(self.root.as_ref().unwrap_or(self).clone())','Some(self.root.as_ref().map_or_else(|| '+C+'::SchemaRc::clone_raw(self), Clone::clone))').replace('let next = Rc::new(Self {','let next: '+C+'::SchemaRc<Layout> = Rc::new(Self {').replace('let next = Rc::new(next);','let next: '+C+'::SchemaRc<Layout> = Rc::new(next).into();').replace('self.layout = Rc::new(layout);','self.layout = Rc::new(layout).into();'))
    # End of the explicit append allocation.
    edit('value/layout/edit.rs',lambda s:s.replace('            flavor: self.flavor,\n        });','            flavor: self.flavor,\n        }).into();'))
    edit('value/layout/host.rs',lambda s:s.replace('Rc::new(layout.copied(true))','Rc::new(layout.copied(true)).into()'))
    edit('value/layout/prototype.rs',lambda s:s.replace('layout.root.as_ref().unwrap_or(layout)','layout.root.as_ref().map_or(layout, |r| &**r)').replace('let root = Rc::new(Layout {','let root: '+C+'::SchemaRc<Layout> = Rc::new(Layout {').replace('            flavor,\n        });','            flavor,\n        }).into();').replace('self.layout = Rc::new(layout);','self.layout = Rc::new(layout).into();').replace('self.layout = Rc::new(next);','self.layout = Rc::new(next).into();').replace('.and_then(Weak::upgrade)', '.and_then(Weak::upgrade).map('+C+'::SchemaRc::from_weak)'))
    edit('value/layout/transitions.rs',lambda s:s.replace('.and_then(Weak::upgrade)', '.and_then(Weak::upgrade).map('+C+'::SchemaRc::from_weak)').replace('child.upgrade())','child.upgrade().map('+C+'::SchemaRc::from_weak))').replace('retained.nodes.push_back(child.clone())','retained.nodes.push_back('+C+'::SchemaRc::clone_raw(child))'))
    inline = (src/'value/layout/value_slots.rs').exists()
    if inline:
        edit('value/layout/edit.rs',lambda s:s.replace('                self.store.values.push(Value::Undefined);','                crate::diagnostics::count!(r2_store_adds);\n                self.store.values.push(Value::Undefined);'))
        edit('value/layout/value_slots.rs',lambda s:s.replace('_ => Self::Heap(values),','_ => { crate::diagnostics::count!(r2_slot_adopted_buffers); Self::Heap(values) },').replace('values.reserve(additional);',C+'::reserve(values, additional);').replace('let mut values = Vec::with_capacity(needed);','let mut values = Vec::with_capacity(needed); '+C+'::capacity::<Value>(0, values.capacity());').replace('let mut values = Vec::with_capacity(4);','let mut values = Vec::with_capacity(4); '+C+'::capacity::<Value>(0, values.capacity());').replace('                values.push(value);\n                Self::Heap(values)','                '+C+'::push(&mut values, value);\n                Self::Heap(values)'))
        edit('value/layout/append_recipe.rs',lambda s:s.replace('        store.values.push(value);','        crate::diagnostics::count!(r2_store_adds);\n        store.values.push(value);'))
        edit('value/layout/sequence.rs',lambda s:s.replace('                values.push(value);','                crate::diagnostics::count!(r2_store_adds);\n                values.push(value);').replace('let prefix = prefix.clone();','let prefix = '+C+'::SchemaRc::clone_raw(prefix);'))
        edit('value/layout/host.rs',lambda s:s.replace('Self::from_slots(layout, ValueSlots::pair(values))','{ crate::diagnostics::count!(r2_inline_pairs); Self::from_slots(layout, ValueSlots::pair(values)) }'))
    else:
        edit('value/layout/host.rs',lambda s:s.replace('        self.values.reserve(capacity);',C+'::reserve(&mut self.values, capacity);'))
        edit('value/layout/host.rs',lambda s:s.replace('        let epoch = if layout.dictionary','        if values.capacity()>0 { crate::diagnostics::count!(r2_slot_adopted_buffers); }\n        let epoch = if layout.dictionary'))
        edit('value/layout/edit.rs',lambda s:s.replace('                self.store.values.push(Value::Undefined);','                crate::diagnostics::count!(r2_store_adds);\n                '+C+'::push(&mut self.store.values, Value::Undefined);'))
    cache = 'bytecode/named_property_cache/stores.rs' if (src/'bytecode/named_property_cache/stores.rs').exists() else 'bytecode/named_property_cache.rs'
    edit(cache,lambda s:count_body(s,'fn write_target(', 'r2_store_ic_probes'))
    # Lease clones and drops are counted regardless of which API created them.
    edit('property/atom.rs',lambda s:s.replace('#[derive(Clone)]\npub(crate) struct KeyLease', 'pub(crate) struct KeyLease').replace('impl KeyLease {','''impl Clone for KeyLease {
        fn clone(&self) -> Self { crate::diagnostics::count!(r2_lease_clones); Self(self.0.clone()) }
    }
    impl Drop for KeyLease { fn drop(&mut self) { crate::diagnostics::count!(r2_lease_drops); } }
    impl KeyLease {''',1).replace('.map(KeyLease)', '.map(|v| { crate::diagnostics::count!(r2_lease_upgrades); KeyLease(v) })'))
else:
    # Base has shared literal schemas but no schema for ordinary constructor
    # stores. Tracking them provides the equivalent ownership count.
    for p in src.rglob('*.rs'):
        s=p.read_text();s=re.sub(r'(?<!&)Rc<ObjectLiteralShape>', C+'::SchemaRc<ObjectLiteralShape>',s);s=s.replace('std::rc::'+C+'::SchemaRc',C+'::SchemaRc').replace('Rc<crate::value::ObjectLiteralShape>',C+'::SchemaRc<crate::value::ObjectLiteralShape>').replace('Rc::clone(shape)',C+'::SchemaRc::clone_raw(shape)');p.write_text(s)
    p=src/'value/object.rs';s=p.read_text();a=s.index('impl ObjectLiteralShape {');b=s.index('enum PropertyStorage',a);part=s[a:b].replace('-> Rc<Self>', '-> '+C+'::SchemaRc<Self>');part=wrap_rc(part,'Rc::new(Self {');s=s[:a]+part+s[b:]
    s=s.replace('            entries.push((key, property));','            crate::diagnostics::count!(r2_store_adds);\n            '+C+'::push(entries, (key, property));')
    s=s.replace('Self(Rc::new(ObjectData {\n            properties: RefCell::new(PropertyStorage::ShapedPair', '{ crate::diagnostics::count!(r2_inline_pairs); Self(Rc::new(ObjectData {\n            properties: RefCell::new(PropertyStorage::ShapedPair') if False else s
    # Count pair adoption, which allocates no separate value buffer.
    s=s.replace('properties: RefCell::new(', 'properties: r2_properties(')
    s += '''
fn r2_properties(storage: PropertyStorage) -> RefCell<PropertyStorage> {
    let capacity = match &storage {
        PropertyStorage::Small { entries } => entries.capacity(),
        PropertyStorage::Shaped { properties, .. } => properties.capacity(),
        PropertyStorage::Dynamic(d) => d.entries.capacity(),
        PropertyStorage::ShapedPair { .. } => { crate::diagnostics::count!(r2_inline_pairs); 0 }
    };
    if capacity>0 { crate::diagnostics::count!(r2_slot_adopted_buffers); }
    RefCell::new(storage)
}
'''
    p.write_text(s)
    edit('property/atom.rs',lambda s:s.replace('#[derive(Clone)]\npub(crate) struct Atom(', 'pub(crate) struct Atom(').replace('impl Atom {', 'impl Clone for Atom { fn clone(&self)->Self { crate::diagnostics::count!(r2_lease_clones); Self(self.0.clone()) } }\nimpl Drop for Atom { fn drop(&mut self) { crate::diagnostics::count!(r2_lease_drops); } }\nimpl Atom {',1).replace('.map(Atom)', '.map(|v| { crate::diagnostics::count!(r2_lease_upgrades); Atom(v) })'))
    # The base cache has a monomorphic write probe under write_hot.
    p=src/'bytecode/named_property_cache.rs'
    if p.exists():
        s=p.read_text();name='fn write_hot(' if 'fn write_hot(' in s else 'fn write('
        s=count_body(s,name,'r2_store_ic_probes');p.write_text(s)
if candidate:
    edit('value/layout/edit.rs',lambda s:s.replace('Rc::new(self.layout.copied(true))','Rc::new(self.layout.copied(true)).into()').replace('Rc::new(self.layout.copied(dictionary))','Rc::new(self.layout.copied(dictionary)).into()'))
    edit('value/layout/host.rs',lambda s:s.replace('self.layout = next;', 'self.layout = next.into();'))
    edit('value/layout/prototype.rs',lambda s:s.replace('let root = Layout::root_shape(&self.layout).clone();', 'let root = '+C+'::SchemaRc::clone_raw(Layout::root_shape(&self.layout));'))
    edit('value/object/literal.rs',lambda s:s.replace('let layout = store.layout().clone();', 'let layout = '+C+'::SchemaRc::clone_raw(store.layout());'))
if not candidate:
    for file in ['value/object/class_fields.rs','value/object/dynamic_storage.rs']:
        edit(file,lambda s:s.replace('entries.reserve(fields.len());',C+'::reserve(entries, fields.len());').replace('entries.push((key.clone(), property));','crate::diagnostics::count!(r2_store_adds); '+C+'::push(entries, (key.clone(), property));').replace('self.entries.push((key, property));','crate::diagnostics::count!(r2_store_adds); '+C+'::push(&mut self.entries, (key, property));'))
# Allocator API requests (all sizes), alongside existing physical block stats.
p=out/'crates/qjs-cli/src/allocator.rs';s=p.read_text();anchor='mod stats {';a=s.index(anchor)+len(anchor)
s=s[:a]+'''
    static REQUESTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static REALLOCS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static REQUEST_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    pub(super) fn request(size: usize, realloc: bool) {
        REQUEST_BYTES.fetch_add(size as u64, std::sync::atomic::Ordering::Relaxed);
        if realloc { REALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
        else { REQUESTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
    }
    pub(super) fn report_requests() {
        eprintln!("R2_ALLOC requests={} reallocs={} requested_bytes={}",
            REQUESTS.load(std::sync::atomic::Ordering::Relaxed),
            REALLOCS.load(std::sync::atomic::Ordering::Relaxed),
            REQUEST_BYTES.load(std::sync::atomic::Ordering::Relaxed));
    }
'''+s[a:]
s=count_body(s,'unsafe fn alloc(&self, layout:', 'r2_objects_created') if False else s
for sig,call in [('unsafe fn alloc(&self, layout:', 'stats::request(layout.size(), false);'),('unsafe fn alloc_zeroed(&self, layout:', 'stats::request(layout.size(), false);'),('unsafe fn realloc(&self, block:', 'stats::request(new_size, true);')]:
 a=s.index(sig);b=s.index('{',a);s=s[:b+1]+'\n'+call+s[b+1:]
a=s.index('pub(crate) fn report_counters()');b=s.index('{',a);s=s[:b+1]+'\n stats::report_requests();'+s[b+1:];p.write_text(s)
print('diagnostic source',revision,out,'candidate' if candidate else 'base')
