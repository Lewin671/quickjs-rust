//! Imported cells must not replace nearer parameter/body/capture bindings.
use super::*;

#[test]
fn import_homonyms_do_not_replace_own_parameters_in_general_expressions() {
    let namespace = run(
        "import {counter,c} from 'dep'; \
        function f(counter){return 'x'+counter;}function g(c){return [c].join();} \
        function rest(c,...counter){return c+':'+counter.join();} \
        function pattern({c}){return 'p'+c;} \
        function captured(c){return (()=>'c'+c)();} \
        export const result=[f(1),g(2),rest(3,4),pattern({c:5}),captured(6),counter,c].join('|');",
        &[("dep", "export let counter=0;export const c='c';")],
    )
    .unwrap();
    assert_eq!(
        export(&namespace, "result"),
        Value::String("x1|2|3:4|p5|c6|0|c".into())
    );
}
