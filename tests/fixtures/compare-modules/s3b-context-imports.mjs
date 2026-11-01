import {value as hidden, set} from './s3b-context-imports-dep.mjs';
import * as ns from './s3b-context-imports-dep.mjs';
let Object;
export {Object};
print([hidden, Function('return typeof hidden')(), (0, eval)('typeof hidden'), typeof Object].join(':'));
set(9);
print(hidden + ':' + ns.value);
