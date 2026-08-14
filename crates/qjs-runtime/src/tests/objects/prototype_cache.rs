//! Invalidation of the per-site cache for prototype-resolved property reads.
//!
//! A site that reads a property the receiver does not own caches *where* the
//! answer came from -- the receiver's direct prototype and the slot within it
//! -- rather than the value.
//!
//! The cache is per *site*, so every test here reads through one function,
//! `read`, both to warm the entry and again after invalidating it. Reading
//! `o.probe` inline after the loop instead would consult a different site's
//! empty cache and pass no matter what the guards do -- which is how the first
//! draft of this file passed with two of the three guards deleted.
use crate::{Value, eval};

/// One shared site, warmed past the point where an entry is installed, then
/// asked again after `mutate` runs.
fn through_one_site(setup: &str, mutate: &str) -> String {
    format!(
        "function read(o) {{ return o.probe; }}
         {setup}
         var before;
         for (var w = 0; w < 40; w++) {{ before = read(subject); }}
         {mutate}
         before + ',' + read(subject);"
    )
}

const KIND: &str = "function Kind() {}
     Kind.prototype.probe = 'proto';
     var subject = new Kind();";

#[test]
fn an_own_property_added_after_warming_shadows_the_cached_prototype_slot() {
    let source = through_one_site(KIND, "subject.probe = 'own';");
    assert_eq!(eval(&source), Ok(Value::String("proto,own".into())));
}

#[test]
fn reassigning_the_prototype_property_is_observed_without_reinstalling() {
    // The entry names a slot, not a value, so an ordinary assignment through
    // the prototype must be visible immediately.
    let source = through_one_site(KIND, "Kind.prototype.probe = 'second';");
    assert_eq!(eval(&source), Ok(Value::String("proto,second".into())));
}

#[test]
fn deleting_the_prototype_property_stops_the_cached_answer() {
    let source = through_one_site(KIND, "delete Kind.prototype.probe;");
    assert_eq!(eval(&source), Ok(Value::String("proto,undefined".into())));
}

#[test]
fn a_property_inserted_before_the_cached_one_does_not_shift_the_answer() {
    // A prototype that gains another property may move the cached slot's
    // contents. The layout guard is what keeps the stale index from being
    // read as if it still named `probe`.
    let source = through_one_site(
        KIND,
        "delete Kind.prototype.probe;
         Kind.prototype.inserted = 'other';
         Kind.prototype.probe = 'reinstalled';",
    );
    assert_eq!(eval(&source), Ok(Value::String("proto,reinstalled".into())));
}

#[test]
fn replacing_the_receivers_prototype_stops_the_cached_answer() {
    let source = through_one_site(KIND, "Object.setPrototypeOf(subject, { probe: 'second' });");
    assert_eq!(eval(&source), Ok(Value::String("proto,second".into())));
}

#[test]
fn a_prototype_property_turned_into_an_accessor_runs_the_getter() {
    let source = format!(
        "function read(o) {{ return o.probe; }}
         {KIND}
         var before;
         for (var w = 0; w < 40; w++) {{ before = read(subject); }}
         var calls = 0;
         Object.defineProperty(Kind.prototype, 'probe', {{
             get: function () {{ calls++; return 'accessor'; }},
             configurable: true
         }});
         before + ',' + read(subject) + ',' + calls;"
    );
    assert_eq!(eval(&source), Ok(Value::String("proto,accessor,1".into())));
}

#[test]
fn one_site_serves_receivers_of_different_prototypes() {
    // The entry is keyed on the holder, so instances of one constructor share
    // it. Four alternating prototypes exercise every cache slot at once.
    let source = "function make(tag) {
             function Kind() {}
             Kind.prototype.probe = tag;
             return new Kind();
         }
         var kinds = [make('a'), make('b'), make('c'), make('d')];
         function read(o) { return o.probe; }
         for (var i = 0; i < 200; i++) { read(kinds[i % 4]); }
         var all = '';
         for (var j = 0; j < 4; j++) { all += read(kinds[j]); }
         all;";
    assert_eq!(eval(source), Ok(Value::String("abcd".into())));
}

#[test]
fn a_receiver_whose_own_property_is_deleted_falls_back_to_the_prototype() {
    let source = "function read(o) { return o.probe; }
         function Kind() { this.probe = 'own'; }
         Kind.prototype.probe = 'proto';
         var subject = new Kind();
         var before;
         for (var w = 0; w < 40; w++) { before = read(subject); }
         delete subject.probe;
         var after;
         for (var v = 0; v < 40; v++) { after = read(subject); }
         before + ',' + after;";
    assert_eq!(eval(source), Ok(Value::String("own,proto".into())));
}

#[test]
fn an_inherited_accessor_is_never_answered_from_the_cache() {
    // The getter must run on every read, so the count is the assertion.
    let source = "function read(o) { return o.probe; }
         function Kind() {}
         var calls = 0;
         Object.defineProperty(Kind.prototype, 'probe', {
             get: function () { calls++; return calls; }
         });
         var subject = new Kind();
         var last = 0;
         for (var i = 0; i < 40; i++) { last = read(subject); }
         last + ',' + calls;";
    assert_eq!(eval(source), Ok(Value::String("40,40".into())));
}
