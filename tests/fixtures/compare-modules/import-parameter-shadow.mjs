import {counter,c} from './import-parameter-shadow-dep.mjs';
function f(counter){return 'x'+counter;}
function g(c){return [c].join();}
function rest(c,...counter){return c+':'+counter.join();}
function pattern({c}){return 'p'+c;}
function captured(c){return (()=>'c'+c)();}
print([f(1),g(2),rest(3,4),pattern({c:5}),captured(6),counter,c].join('|'));
