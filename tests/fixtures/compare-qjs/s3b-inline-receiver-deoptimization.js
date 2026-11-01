(function () {
function V() { this.lv = [0, 0, 0]; }
V.prototype.process = function(det, lus) {
  var i, dv, dm = 0.0;
  if (typeof(lus) == 'undefined') lus = det.last;
  for (i = 0; i < 3; i++) { dv = det.a[i] / det.b[i]; if (dv > dm) dm = dv; }
  if (dm <= 0) dm = 1.0;
  for (i = 0; i < 3; i++) {
    dv = det.a[i] / det.b[i];
    if (dv != dv) dv = 0;
    if (dv > 1.0) { dv -= 1.0; if (dv > 1.0) dv = 1.0;
      if (dv > this.lv[i]) this.lv[i] = dv;
      else if (det.bpm) this.lv[i] -= (this.lv[i] - dv) * lus * (1.0 / det.bpm) * 3.0;
    } else { if (det.bpm) this.lv[i] -= (lus / det.bpm) * 2.0; }
    if (this.lv[i] < 0 || this.lv[i] != this.lv[i]) this.lv[i] = 0;
  }
};
var bd = { a: [3, 2, 1], b: [1, 1, 1], last: 5, bpm: 120 };
var missing = new V(), explicit = new V();
function calcMissing() { missing.process(bd); }
function calcExplicit() { explicit.process(bd, 5); }
for (var k = 0; k < 2000; k++) { calcMissing(); calcExplicit(); }
return JSON.stringify([missing.lv, explicit.lv]);
})()
