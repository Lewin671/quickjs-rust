// Chain-focused IC differential fuzzer (compare ICs on/off).
var SEED = __SEED__;
var state = SEED >>> 0 || 1;
function rnd(n) { state ^= state << 13; state >>>= 0; state ^= state >>> 17; state ^= state << 5; state >>>= 0; return state % n; }
var out = [];
function log(x) { out[out.length] = x; }
var IDS = new Map();
function show(v) {
  if (v === undefined) return 'u';
  if (v === null) return 'n';
  var t = typeof v;
  if (t === 'function' || t === 'object') { var id = IDS.get(v); return (t === 'function' ? 'F' : 'O') + (id === undefined ? '?' : id); }
  return String(v);
}
var protos = [], recs = [];
function newProto(i) {
  var k = rnd(6), p;
  if (k === 0) p = {};
  else if (k === 1) p = {a: 'pa' + i};
  else if (k === 2) p = [i];
  else if (k === 3) p = function () {};
  else if (k === 4) { p = {}; for (var j = 0; j < 66; j++) p['z' + j] = j; }
  else p = {b: 'pb' + i, c: 'pc' + i};
  IDS.set(p, 'p' + i);
  return p;
}
for (var i = 0; i < 7; i++) protos[i] = newProto(i);
// link protos into random chains
for (var i = 1; i < 7; i++) { try { Object.setPrototypeOf(protos[i], protos[rnd(i)]); } catch (e) {} }
function newRec(i) {
  var k = rnd(5), r;
  if (k === 0) r = Object.create(protos[rnd(7)]);
  else if (k === 1) { r = [1, 2]; Object.setPrototypeOf(r, protos[rnd(7)]); }
  else if (k === 2) { r = function () {}; Object.setPrototypeOf(r, protos[rnd(7)]); }
  else if (k === 3) { r = {x: i}; Object.setPrototypeOf(r, protos[rnd(7)]); }
  else r = Object.create(protos[rnd(7)], {y: {value: 1, writable: true, configurable: true}});
  IDS.set(r, 'r' + i);
  return r;
}
for (var i = 0; i < 10; i++) recs[i] = newRec(i);
// sites
function ra(o) { return o.a; }
function rb(o) { return o.b; }
function rc(o) { return o.c; }
function rl(o) { return o.length; }
function rp(o) { return o.prototype; }
function loopA(o, n) { var s = ''; for (var i = 0; i < n; i++) { s = o.a; } return s; }
function loopB(list) { var s = ''; for (var i = 0; i < list.length; i++) { s += show(list[i].b) + ','; } return s; }
function wa(o, v) { o.a = v; }
function inc(o) { o.c += '+'; return o.c; }
var M = { m: function () { return this.a; } };
var READ = [ra, rb, rc, rl, rp];
var KEYS = ['a', 'b', 'c', 'length', 'prototype'];
var gcount = 0; var SITES = [];
function anyObj() { return rnd(3) === 0 ? protos[rnd(protos.length)] : recs[rnd(recs.length)]; }
for (var step = 0; step < 3000; step++) {
  var c = rnd(100);
  try {
    if (c < 20) {
      var f = rnd(READ.length);
      var s = '';
      for (var j = 0; j < recs.length; j++) s += show(READ[f](recs[j])) + ',';
      log('R' + f + ':' + s);
    } else if (c < 40) {
      // per-receiver monomorphic sites, refreshed now and then
      var j = rnd(recs.length), f = rnd(KEYS.length);
      if (!SITES[j] || rnd(30) === 0) SITES[j] = KEYS.map(function (k) { return new Function('o', 'return o.' + k + ';'); });
      var s = '';
      for (var t = 0; t < 3; t++) s += show(SITES[j][f](recs[j])) + ',';
      log('r' + j + KEYS[f] + ':' + s);
    } else if (c < 45) {
      var o = anyObj();
      log('L' + show(o) + loopA(o, 1 + rnd(20)));
    } else if (c < 48) {
      log('B' + loopB(recs));
    } else if (c < 52) {
      var o = anyObj(); M.m.call(o); log('M' + show(o) + show(M.m.call(o)));
    } else if (c < 60) {
      var p = protos[rnd(protos.length)]; var key = KEYS[rnd(3)];
      p[key] = 'v' + step; log('S' + show(p) + key);
    } else if (c < 66) {
      var p = anyObj(); var key = KEYS[rnd(3)]; log('X' + show(p) + key + delete p[key]);
    } else if (c < 72) {
      var p = anyObj(); var key = KEYS[rnd(KEYS.length)]; var g = 'g' + (gcount++);
      var kind = rnd(3);
      if (kind === 0) Object.defineProperty(p, key, {get: function () { return g; }, configurable: true});
      else if (kind === 1) Object.defineProperty(p, key, {value: g, writable: !!rnd(2), configurable: true});
      else Object.defineProperty(p, key, {value: g, writable: true, configurable: true, enumerable: false});
      log('D' + show(p) + key + kind);
    } else if (c < 80) {
      var o = anyObj(); var np = rnd(8) === 0 ? null : protos[rnd(protos.length)];
      log('P' + show(o) + show(np) + Reflect.setPrototypeOf(o, np));
    } else if (c < 84) {
      var o = recs[rnd(recs.length)]; wa(o, 'w' + step); log('W' + show(o));
    } else if (c < 87) {
      var o = anyObj(); log('I' + show(o) + inc(o));
    } else if (c < 90) {
      var o = anyObj(); var ks = ''; for (var k in o) ks += k + ';'; log('K' + show(o) + ks);
    } else if (c < 92) {
      var o = anyObj(); var z = rnd(3); if (z === 0) Object.freeze(o); else if (z === 1) Object.seal(o); else Object.preventExtensions(o); log('Z' + show(o) + z);
    } else if (c < 96) {
      var idx = rnd(recs.length); recs[idx] = newRec(recs.length + step); log('N' + idx);
    } else {
      var idx = rnd(protos.length); var np = newProto(protos.length + step);
      try { Object.setPrototypeOf(np, protos[rnd(protos.length)]); } catch (e) {}
      protos[protos.length] = np; log('Q' + show(np));
    }
  } catch (e) {
    log('E' + (e && e.constructor && e.constructor.name));
  }
}
// Warm constants before each write route, including the instability threshold.
var constantProto = {m: function(){return 0}, text:'s'};
var constantRec = Object.create(constantProto);
function constantRead(){return constantRec.m() + ':' + constantRec.text;}
for(var overwrite=0;overwrite<12;overwrite++) {
  for(var hot=0;hot<40;hot++) constantRead();
  var replacement = (function(v){return function(){return v}})(rnd(10000));
  if(overwrite%4===0) constantProto.m=replacement;
  else if(overwrite%4===1) Reflect.set(constantProto,'m',replacement);
  else if(overwrite%4===2) Object.defineProperty(constantProto,'m',{value:replacement});
  else Object.assign(constantProto,{m:replacement});
  constantProto.text+='x'; log('constant:'+constantRead());
}
for(var access=0;access<6;access++) {
  Object.defineProperty(constantProto,'a',{get:(function(v){return function(){return v}})(rnd(1000)),configurable:true});
  for(var hot=0;hot<40;hot++) ra(constantRec);
  log('accessor:'+ra(constantRec));
}
var h = 0; var all = out.join('\n');
for (var i = 0; i < all.length; i++) h = (h * 31 + all.charCodeAt(i)) | 0;
print(h, out.length);
if (typeof DUMP !== 'undefined') print(all);
