//! String/RegExp protocols keep one shared subject: coercions return the
//! same handle, every exec and callback receives it, and its memoized
//! metadata is established once instead of once per call.

use crate::{JsString, Value, eval};

/// Evaluates `source`, whose completion value is an array of strings, and
/// returns those handles.
fn string_handles(source: &str) -> Vec<JsString> {
    let Ok(Value::Array(array)) = eval(source) else {
        panic!("expected an array completion from {source}");
    };
    (0..array.len())
        .map(|index| match array.get(index) {
            Some(Value::String(value)) => value,
            other => panic!("element {index} is not a string: {other:?}"),
        })
        .collect()
}

fn assert_all_share_first(handles: &[JsString], expected_len: usize) {
    assert_eq!(handles.len(), expected_len);
    for (index, handle) in handles.iter().enumerate().skip(1) {
        assert!(
            JsString::ptr_eq(&handles[0], handle),
            "element {index} ({handle:?}) is a copy of the subject"
        );
    }
}

fn string_completion(source: &str) -> JsString {
    match eval(source) {
        Ok(Value::String(value)) => value,
        other => panic!("expected a string completion from {source}: {other:?}"),
    }
}

#[test]
fn string_coercion_preserves_hook_order() {
    assert_eq!(
        eval(
            "var log = [];\
             var needle = { toString() { log.push('needle'); return 'b'; } };\
             var position = { valueOf() { log.push('position'); return 0; } };\
             var found = 'abc'.indexOf(needle, position);\
             var re = /b/g;\
             Object.defineProperty(re, 'flags', { get() { log.push('flags'); return 'g'; } });\
             var input = { toString() { log.push('input'); return 'abcb'; } };\
             var matched = RegExp.prototype[Symbol.match].call(re, input).length;\
             var primitive = { [Symbol.toPrimitive](hint) { log.push('primitive:' + hint); return 'x'; },\
                               toString() { log.push('unexpected'); return 'y'; } };\
             var included = 'axb'.includes(primitive);\
             var searchInput = { toString() { log.push('search-input'); return 'zzb'; } };\
             var searched = RegExp.prototype[Symbol.search].call(/b/, searchInput);\
             [found, matched, included, searched, log.join(',')].join('|');"
        ),
        Ok(Value::String(
            "1|2|true|2|needle,position,input,flags,primitive:string,search-input"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval(
            "var caught = false;\
             try { RegExp.prototype[Symbol.match].call(/a/, Symbol('s')); }\
             catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn boxed_string_methods_share_data() {
    let handles = string_handles(
        "var s = 'xy'.repeat(64);\
         var boxed = new String(s);\
         var seen = [];\
         var re = /y/g;\
         re.exec = function (value) { seen.push(value); return null; };\
         boxed.match(re);\
         boxed.search(re);\
         boxed.replace(re, '');\
         var fake = { lastIndex: 0, exec(value) { seen.push(value); return null; } };\
         var species = /y/g;\
         species.constructor = { [Symbol.species]: function () { return fake; } };\
         String.prototype.matchAll.call(boxed, species).next();\
         [boxed.valueOf(), boxed.toString()].concat(seen);",
    );
    assert_all_share_first(&handles, 6);
}

#[test]
fn match_all_keeps_subject_across_next() {
    let handles = string_handles(
        "var s = 'ab'.repeat(8);\
         var seen = [];\
         var fake = { lastIndex: 0, calls: 0, exec(value) {\
           seen.push(value);\
           return this.calls++ < 3 ? { 0: 'b', index: 2 * this.calls - 1, length: 1 } : null;\
         } };\
         var re = /b/g;\
         re.constructor = { [Symbol.species]: function () { return fake; } };\
         var count = 0;\
         for (var match of s.matchAll(re)) count++;\
         if (count !== 3) throw new Error('count ' + count);\
         [s].concat(seen);",
    );
    assert_all_share_first(&handles, 5);

    // The builtin iterator execs the subject itself, so the matcher view it
    // indexes is memoized on that subject rather than on per-call copies.
    let subject = string_completion(
        "var s = 'ab'.repeat(32); var n = 0; for (var m of s.matchAll(/b/g)) n++;\
         if (n !== 32) throw new Error('n ' + n); s;",
    );
    subject.matcher_view_with(false, |_| {
        panic!("matchAll did not exec the shared subject")
    });
}

#[test]
fn regexp_protocols_build_the_subject_view_once() {
    for source in [
        "var s = 'ab'.repeat(32); if (s.match(/b/g).length !== 32) throw 0; s;",
        "var s = 'ab'.repeat(32); if (s.search(/b/) !== 1) throw 0; s;",
        "var s = 'ab'.repeat(32); var re = /b/g; re.exec = function (v) { return RegExp.prototype.exec.call(this, v); };\
         if (s.replace(re, 'c') !== 'ac'.repeat(32)) throw 0; s;",
        "var s = 'ab'.repeat(32); if (s.split(/b/).length !== 33) throw 0; s;",
    ] {
        let subject = string_completion(source);
        subject.matcher_view_with(false, |_| panic!("the subject was copied: {source}"));
    }
    // Unicode mode keys the view by mode; a non-ASCII subject keeps its own.
    let wide = string_completion(
        "var s = '\\u00e9b'.repeat(16); if (s.match(/b/gu).length !== 16) throw 0; s;",
    );
    wide.matcher_view_with(true, |_| panic!("the wide subject was copied"));
}

#[test]
fn custom_exec_observes_every_lookup() {
    assert_eq!(
        eval(
            "var log = [];\
             var calls = 0;\
             var re = /b/g;\
             Object.defineProperty(re, 'exec', { get() {\
               log.push('exec');\
               return function (v) { calls++; return RegExp.prototype.exec.call(this, v); };\
             } });\
             var count = 'abab'.match(re).length;\
             var replaced = 'abab'.replace(re, 'x');\
             var searched = 'abab'.search(re);\
             [count, replaced, searched, log.length, calls].join(':');"
        ),
        Ok(Value::String("2:axax:1:7:7".to_owned().into()))
    );
    // A RegExp-like object observes every lastIndex read and write while
    // empty matches advance through an ASCII subject.
    assert_eq!(
        eval(
            "function regexpLike(log) {\
               return { flags: 'g', li: 0, n: 0,\
                 get lastIndex() { log.push('get ' + this.li); return this.li; },\
                 set lastIndex(v) { log.push('set ' + v); this.li = v; },\
                 exec(v) { log.push('exec'); return this.n++ < 2 ? { 0: '', index: this.li, length: 1 } : null; } };\
             }\
             var matchLog = [];\
             var matched = RegExp.prototype[Symbol.match].call(regexpLike(matchLog), 'aaa').length;\
             var replaceLog = [];\
             var replaced = RegExp.prototype[Symbol.replace].call(regexpLike(replaceLog), 'aaa', '-');\
             [matched, matchLog.join(','), replaced, replaceLog.join(',')].join('|');"
        ),
        Ok(Value::String(
            "2|set 0,exec,get 0,set 1,exec,get 1,set 2,exec|-a-aa|set 0,exec,get 0,set 1,exec,get 1,set 2,exec"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn functional_replace_shares_input() {
    let handles = string_handles(
        "var s = 'a,b,c,'.repeat(8);\
         var seen = [];\
         s.replaceAll(',', function (m, p, input) { seen.push(input); return ';'; });\
         s.replace(',', function (m, p, input) { seen.push(input); return ';'; });\
         s.replace(/,/g, function (m, p, input) { seen.push(input); return ';'; });\
         [s].concat(seen.slice(0, 2), seen.slice(-1));",
    );
    assert_all_share_first(&handles, 4);
    assert_eq!(
        eval(
            "var s = 'x-y-z';\
             [s.replaceAll('-', \"[$`|$&|$']\"), s.replace('-', \"<$'>\"), s.replaceAll('-', '$$')].join(' ');"
        ),
        Ok(Value::String(
            "x[x|-|y-z]y[x-y|-|z]z x<y-z>y-z x$y$z".to_owned().into()
        ))
    );
}

#[test]
fn ascii_search_offset_does_not_rescan_subject() {
    // A cursor loop over one retained ASCII document classifies it once; the
    // boundary of every later search answers from that memoized flag.
    let subject = string_completion(
        "var s = 'field,'.repeat(256); var cursor = 0, count = 0;\
         while ((cursor = s.indexOf(',', cursor)) !== -1) { cursor++; count++;\
           if (!s.startsWith('field', cursor) && cursor !== s.length) throw 0;\
           if (!s.includes(',', cursor - 1) || !s.endsWith(',', cursor)) throw 1; }\
         if (count !== 256) throw 2; s;",
    );
    assert_eq!(subject.ascii_scan_count(), 1);
    // Positions keep their existing coordinates on both branches.
    assert_eq!(
        eval(
            "['abcabc'.indexOf('c', 3), 'abc'.indexOf('', 9), 'abc'.startsWith('c', 2),\
              'abc'.endsWith('a', 1), 'abc'.includes('a', 1), '\\u00e9a\\u00e9a'.indexOf('a', 2),\
              'caf\\u00e9!'.endsWith('\\u00e9', 4)].join(',');"
        ),
        Ok(Value::String(
            "5,3,true,true,false,3,true".to_owned().into()
        ))
    );
}
