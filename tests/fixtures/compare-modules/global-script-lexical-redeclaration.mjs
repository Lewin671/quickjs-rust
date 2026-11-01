const evalScript = typeof __quickjsRustEvalScript === 'function'
  ? __quickjsRustEvalScript : (await import('qjs:std')).evalScript;
globalThis.p=1;
evalScript('let p=10');
let result;
try { result=evalScript('var side;let p=2;5'); }
catch(e) { result=e.name; }
print(result+':'+p+':'+globalThis.p+':'+Object.prototype.hasOwnProperty.call(globalThis,'side'));
