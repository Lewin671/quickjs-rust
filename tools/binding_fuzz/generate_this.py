#!/usr/bin/env python3
"""Seeded generator: implicit bindings (this / arguments / new.target) read
inside member and compound assignments across branches and loops, reached
through several call paths (top level, JS caller, hot JS caller loop, new,
call/apply, arrow capture, method on prototype).

Adapted from reviewer A's gen_this.py, supplied with T077 round 3.
usage: python3 -m tools.binding_fuzz.generate_this SEED > prog.js
"""
import json
import random
import sys


class G:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.np = self.r.randrange(1, 4)
        self.params = ['a', 'b', 'c'][:self.np]
        self.strict = self.r.random() < 0.2
        self.arrow_ok = True
        self.lean = self.r.random() < 0.75
        self.pure = self.lean and self.r.random() < 0.7

    def pick(self, xs):
        return self.r.choice(xs)

    def ok(self, x):
        return not (self.pure and ('arguments' in x or 'new.target' in x or '=>' in x))

    def pk(self, f):
        for _ in range(50):
            x = f()
            if self.ok(x):
                return x
        return '1'

    def idx(self):
        return self.pick(['i', 'i', '0', '1', 'j'])

    def lhs(self):
        return self.pk(self._lhs)

    def _lhs(self):
        return self.pick([
            'this.v[%s]' % self.idx(), 'this.v[%s]' % self.idx(), 'this.w', 'this.o.q',
            'this["w"]', 't', self.pick(self.params), 'arguments[%d]' % self.r.randrange(3),
        ])

    def num(self):
        return self.pick(['1', '2', '0.5', '3.0', '1.0', '0'])

    def e(self, d=0):
        return self.pk(lambda: self._e(d))

    def _e(self, d=0):
        k = self.r.randrange(22 if d < 2 else 10)
        if k < 2:
            return self.num()
        if k < 3:
            return 'a.k'
        if k < 4:
            return self.pick(self.params)
        if k < 5:
            return 't'
        if k < 7:
            return 'this.v[%s]' % self.idx()
        if k < 8:
            return 'this.w'
        if k < 9:
            return 'arguments.length'
        if k < 10:
            return '(arguments[0] && arguments[0].k)'
        if k < 13:
            return '(%s %s %s)' % (self.e(d + 1), self.pick(['-', '+', '*', '/']), self.e(d + 1))
        if k < 14:
            return '(%s - %s) * %s * (1.0 / a.k) * 3.0' % (self.e(d + 1), self.e(d + 1), self.e(d + 1))
        if k < 15:
            return '(new.target ? 1 : 0)'
        if k < 16:
            return '(typeof this === "object" ? 1 : 0)'
        if k < 17:
            return '(this === S ? 1 : 0)'
        if k < 18:
            return '(() => this.w)()' if not self.lean else 'this.w'
        if k < 19:
            return '(() => arguments.length)()' if not self.lean else 'arguments[1]'
        if k < 20:
            return 'this.o.q'
        return '%s[%s]' % (self.pick(['this.v', 'a.arr']), self.idx())

    def cond(self):
        return self.pk(self._cond)

    def _cond(self):
        return self.pick([
            'a.k', 'a', self.pick(self.params), 'a.bpm', '%s != %s' % ((self.e(),) * 2),
            '%s > %s' % (self.e(), self.e()), '%s < 0' % self.e(), 'typeof %s == "undefined"' % self.pick(self.params),
            'arguments.length > %d' % self.r.randrange(3), 'new.target', 'this.v[i] != this.v[i]', 't > 1', 'i % 2',
            '%s %s %s' % (self.pick(['this.w', 'this.v[i]', 'this.o.q', 'this["w"]', 'arguments[0]', 'arguments.length', 'new.target', 'a.k', 't']), self.pick(['==', '===', '!=', '!==', '<', '>', '<=', '>=']), self.e()),
            '%s %s %s' % (self.pick(['this.w', 'this.v[i]', 'this.o.q']), self.pick(['==', '===', '!=', '!==']), self.num()),
        ])

    def rec(self, x):
        return ('acc += %s;' % x) if self.lean else ('R.push(%s);' % x)

    def stmt(self, d):
        return self.pk(lambda: self._stmt(d))

    def _stmt(self, d):
        k = self.r.randrange(16 if d < 3 else 9)
        if k < 3:
            return '%s %s %s;' % (self.lhs(), self.pick(['=', '-=', '+=', '*=', '-=']), self.e())
        if k < 4:
            return '%s%s;' % (self.lhs(), self.pick(['++', '--']))
        if k < 5:
            return self.rec(self.pick(['this === S', 'typeof this', 'arguments.length', 'arguments[0] === a', 'new.target === undefined', self.e(), 'this.v[i]', '%s' % self.pick(self.params)]))
        if k < 6:
            p = self.pick(self.params)
            return 'if (typeof(%s) == "undefined") %s = a.last;' % (p, p)
        if k < 7:
            return 't = %s;' % self.e()
        if k < 8:
            return 'if (%s) %s' % (self.cond(), self.stmt(d + 1))
        if k < 9:
            return self.rec(self.e())
        if k < 11:
            return 'if (%s) {\n%s\n} else {\n%s\n}' % (self.cond(), self.body(d + 1), self.body(d + 1))
        if k < 13:
            return 'for (i = 0; i < 3; i++) {\n%s\n}' % self.body(d + 1)
        if k < 14:
            return self.pick(['for (var j = 0; j < 2; j++) {\n%s\n}', 'for (let j = 0; j < 2; j++) {\n%s\n}', 'j = 0; do {\n%s\n} while (++j < 2);']) % self.body(d + 1)
        if k < 15:
            return 'while (t < 2 && n++ < 4) {\n%s\nt += 1;\n}' % self.body(d + 1)
        return 'if (%s) {\n%s\n} else if (%s) {\n%s\n}' % (self.cond(), self.body(d + 1), self.cond(), self.body(d + 1))

    def body(self, d):
        return '\n'.join(self.stmt(d) for _ in range(self.r.randrange(1, 4)))

    def args(self):
        n = self.r.randrange(0, 4)
        base = ['bd', '2', '0.5']
        return ', '.join(base[:n])

    def program(self):
        ps = ', '.join(self.params)
        body = '\n'.join(self.stmt(0) for _ in range(self.r.randrange(3, 9)))
        decl = 'var acc = 0, i = 0, j = 0, t = 0.0, n = 0, dm = 0.0;' if self.lean else 'var R = [], i = 0, j = 0, t = 0.0, n = 0, dm = 0.0;'
        fn_body = '%s%s\n%s\nreturn %s;' % ('"use strict";\n' if self.strict else '', decl, body, 'acc' if self.lean else 'R')
        out = []
        out.append('function show(x) { try { return JSON.stringify(x); } catch (e) { return "<cyc>"; } }')
        out.append('function run(l, f) { var r; try { r = show(f()); } catch (e) { r = "throws " + (e && e.constructor && e.constructor.name); } print(l + ": " + r); }')
        out.append('var bd = { k: 2, a: [3, 2, 1], arr: [1, 2, 3], b: [1, 1, 1], last: 5, bpm: 120 };')
        out.append('function V() { this.v = [0, 1, 2]; this.w = 1; this.o = {q: 1}; }')
        out.append('var S;')
        out.append('V.prototype.p = function (%s) {\n%s\n};' % (ps, fn_body))
        out.append('function P(%s) {\nthis.v = [0, 1, 2]; this.w = 1; this.o = {q: 1}; S = this;\n%s\nthis.R = R;\n}' % (ps, fn_body.replace('return R;', '').replace('return acc;', 'var R = acc;')))
        out.append('var plain = function (%s) {\n%s\n};' % (ps, fn_body))
        calls = []
        for ci in range(6):
            a = self.args()
            kind = self.r.randrange(8)
            if kind == 0:
                calls.append('run("top%d", function () { var o = new V(); S = o; var r = o.p(%s); return [r, o.v, o.w, o.o]; });' % (ci, a))
            elif kind == 1:
                calls.append('function calc%d(o) { return o.p(%s); }\nrun("jscall%d", function () { var o = new V(); S = o; var r = calc%d(o); return [r, o.v, o.w, o.o]; });' % (ci, a, ci, ci))
            elif kind == 2:
                calls.append('var o%d = new V(); var calc%d = function () { o%d.p(%s); };\nrun("hot%d", function () { S = o%d; for (var k = 0; k < 40; k++) calc%d(); return [o%d.v, o%d.w, o%d.o]; });' % (ci, ci, ci, a, ci, ci, ci, ci, ci, ci))
            elif kind == 3:
                calls.append('run("new%d", function () { var x = new P(%s); return [x.R, x.v, x.w, x.o]; });' % (ci, a))
            elif kind == 4:
                calls.append('function mk%d() { return new P(%s); }\nrun("jsnew%d", function () { var x = mk%d(); return [x.R, x.v, x.w, x.o]; });' % (ci, a, ci, ci))
            elif kind == 5:
                calls.append('run("call%d", function () { var o = new V(); S = o; var r = plain.call(o, %s); return [r, o.v, o.w]; });' % (ci, a) if a else 'run("call%d", function () { var o = new V(); S = o; var r = plain.call(o); return [r, o.v, o.w]; });' % ci)
            elif kind == 6:
                calls.append('function outer%d() { var o = new V(); S = o; var f = () => o.p(%s); return [f(), o.v]; }\nrun("arrow%d", function () { return outer%d(); });' % (ci, a, ci, ci))
            else:
                calls.append('function apl%d(o) { return V.prototype.p.apply(o, [%s]); }\nrun("apply%d", function () { var o = new V(); S = o; var r = apl%d(o); for (var k = 0; k < 5; k++) apl%d(o); return [r, o.v, o.w]; });' % (ci, a, ci, ci, ci))
        out.extend(calls)
        return '\n'.join(out) + '\n'


if __name__ == '__main__':
    sys.stdout.write(G(int(sys.argv[1])).program())
