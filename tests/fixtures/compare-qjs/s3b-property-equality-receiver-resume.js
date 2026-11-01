(function(){var out=[];function print(){out.push(Array.prototype.join.call(arguments," "));}
function p() { var r = 0; for (var i = 0; i < 3; i++) { if (this.w == 0) r += this.v; } return r; }
var o = {w: 0, v: 2, p: p};
function calc() { return o.p(); }
try { print(calc()); } catch (e) { print('throws ' + e.constructor.name); }
function P() { this.w = 5; for (var i = 0; i < 3; i++) { if (this.w != 1) this.w = 0; } }
function mk() { return new P(); }
try { print(mk().w); } catch (e) { print('throws ' + e.constructor.name); }
class C { constructor() { this.w = 0; this.v = 2; } m() { var r = 0; for (var i = 0; i < 3; i++) { r += this.w === 0 ? this.v : 0; } return r; } }
try { print([1].map(function () { return new C().m(); })[0]); } catch (e) { print('throws ' + e.constructor.name); }
var G = [];
function p3(a) { var r = 0; for (var i = 0; i < 4; i++) { if (a.k == 1) r += (this === o3); G.push(r); } return r; }
var o3 = {p3: p3}; function calc3() { return o3.p3({k: 1}); }
print(calc3(), G.join()); // NG/base: 4 1,2,3,4   candidate: 3 1,1,2,3
function p4(a) { var r = 0; for (var i = 0; i < 3; i++) { if (a.k == 1) r += this.v; } return r; }
var o4 = {v: 2, p4: p4}; function calc4() { return o4.p4({k: 1}); }
try { print(calc4()); } catch (e) { print('throws ' + e.constructor.name); } // NG/base: 6  candidate: throws TypeError
return out.join("|");})()
