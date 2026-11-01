(function(){function f(a,b){a=3;Object.freeze(arguments);a=5;b=6;return [a,b,arguments[0],arguments[1],Object.isFrozen(arguments)].join();}
function proxy(a){Object.freeze(new Proxy(arguments,{}));a=5;return arguments[0];}
function sealed(a){Object.seal(arguments);a=5;return arguments[0];}
return [f(1,2),proxy(1),sealed(1)].join('|');})()
