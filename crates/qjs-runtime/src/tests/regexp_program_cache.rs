//! RegExp objects with the same source and flags share one compiled program
//! per realm; these cover what must stay per object or per flag set.

use crate::{Value, eval};

fn string(source: &str) -> Value {
    Value::String(source.into())
}

#[test]
fn objects_sharing_a_program_keep_their_own_last_index() {
    assert_eq!(
        eval(
            "var results = [];
             for (var i = 0; i < 3; i++) {
                 var re = /a(b)?/g;
                 re.lastIndex = i;
                 var m = re.exec('xab-ab');
                 results.push(m ? m.index + ':' + re.lastIndex + ':' + m[1] : 'none');
             }
             var shared1 = /a/g, shared2 = /a/g;
             shared1.exec('aa');
             results.push(shared1.lastIndex + '/' + shared2.lastIndex);
             results.join();"
        ),
        Ok(string("1:3:b,1:3:b,4:6:b,1/0"))
    );
}

#[test]
fn the_same_source_under_different_flags_is_a_different_program() {
    assert_eq!(
        eval(
            "[/a.c/.test('A\\nC'), /a.c/i.test('A\\nC'), /a.c/is.test('A\\nC'),
              /^b/.test('a\\nb'), /^b/m.test('a\\nb'),
              /\\u{61}/.test('a'), /\\u{61}/u.test('a')].join();"
        ),
        Ok(string("false,false,true,false,true,false,true"))
    );
}

#[test]
fn an_invalid_pattern_fails_on_every_construction() {
    assert_eq!(
        eval(
            "var failures = 0;
             for (var i = 0; i < 3; i++) {
                 try { new RegExp('(', ''); } catch (e) { if (e instanceof SyntaxError) failures++; }
                 try { new RegExp('a', 'gg'); } catch (e) { if (e instanceof SyntaxError) failures++; }
             }
             failures;"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn many_distinct_patterns_still_match_after_the_cache_is_cleared() {
    assert_eq!(
        eval(
            "var ok = 0;
             for (var round = 0; round < 2; round++) {
                 for (var i = 0; i < 150; i++) {
                     var re = new RegExp('x' + i + 'y');
                     if (re.test('<x' + i + 'y>') && !re.test('x' + (i + 1) + 'y')) ok++;
                 }
             }
             ok;"
        ),
        Ok(Value::Number(300.0))
    );
}

#[test]
fn replace_shares_the_program_with_exec() {
    assert_eq!(
        eval(
            "var re = /(\\d+)/g;
             var first = 'a1b22c333'.replace(re, '[$1]');
             var second = re.exec('zz44');
             first + '|' + second[1] + '|' + 'q5'.replace(/(\\d+)/g, '<$1>');"
        ),
        Ok(string("a[1]b[22]c[333]|44|q<5>"))
    );
}
