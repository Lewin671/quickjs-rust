(function(){var r=[];
r.push(eval("var w = 0; while (w++ < 2) { try { throw 0 } catch { 2; if (true) break; } }"));
r.push(eval("var w = 0; while (w++ < 2) { try { throw 0 } catch { 2; with ({}) break; } }"));
r.push(eval("var w = 0; while (w++ < 2) { try { throw 0 } catch { 2; if (true) continue; } }"));
r.push(eval("L: { try { throw 0 } catch { 2; if (true) break L; } }"));
r.push(eval("switch (1) { case 1: try { throw 0 } catch { 2; if (true) break; } }"));
r.push(eval("var w = 0; while (w++ < 2) { try { 2; if (true) break; } finally { } }"));
r.push(eval("var w = 0; while (w++ < 2) { try { throw 0 } catch { 2; break; } }"));
r.push(eval("7; try { 8 } catch { }"));
r.push(eval("7; try { throw 0 } catch { }"));
return r.map(String).join('|');})()
