#!/usr/bin/env python3
"""Seeded generator: global Name assignments whose RHS mutates globalThis.

Each of five global names starts as a var, an own data or accessor property,
a script lexical, an eval var or nothing; every third seed also puts data,
read-only or accessor properties of two names on Object.prototype. Plain,
compound, chained, destructuring, for-in/of, update and logical assignments
then run at top level and in sloppy and strict functions called repeatedly,
with RHS effects that delete or redefine the property, declare a lexical or
var through another script or eval, or change the prototype. Snapshots record
every name's descriptor, its value and a side-effect log, so a store that
misses PutValue/SetMutableBinding semantics shows up as an observation.

Adapted from the T077 re-review's genA.py; seeds produce the same programs.
usage: python3 -m tools.binding_fuzz.generate_global SEED > prog.js
"""
import random
import sys

NAMES = ['gv', 'gw', 'gx', 'gy', 'gz']


class GlobalStoreGen:
    def __init__(self, seed):
        self.r = random.Random(seed * 7919 + 1)
        # One third of programs pollute Object.prototype.
        self.proto = seed % 3 == 0
        self.lexicals = set()

    def effect(self):
        r = self.r
        n = r.choice(NAMES)
        v = r.randrange(100)
        k = r.randrange(16 if self.proto else 13)
        if k == 0:
            return 'delete globalThis.%s' % n
        if k == 1:
            return 'globalThis.%s = %d' % (n, v)
        if k == 2:
            return 'Object.defineProperty(globalThis, "%s", {value: %d, writable: %s, configurable: true})' % (
                n, v, r.choice(['true', 'false']))
        if k == 3:
            return ('Object.defineProperty(globalThis, "%s", {get(){ LOG.push("g2%s"); return %d; }, '
                    'set(x){ LOG.push("s2%s" + x); }, configurable: true})') % (n, n, v, n)
        if k == 4:
            return '(function(){ try { ES("let %s = %d"); } catch (e) { LOG.push("ES!" + e.name); } })()' % (n, v)
        if k == 5:
            return '(0,eval)("var %s = %d")' % (n, v)
        if k == 6:
            return 'ES("var %s = %d")' % (n, v) if n not in self.lexicals else 'LOG.push("x")'
        if k == 7:
            return '(delete globalThis.%s, globalThis.%s = %d)' % (n, n, v)
        if k == 8:
            return 'LOG.push(typeof %s)' % n
        if k == 9:
            return 'Object.defineProperty(globalThis, "%s", {writable: false})' % n
        if k == 10:
            return '(function(){ try { %s = %d; } catch (e) { LOG.push("in!" + e.name); } })()' % (n, v)
        if k == 11:
            return '(function(){ "use strict"; try { %s = %d; } catch (e) { LOG.push("ins!" + e.name); } })()' % (n, v)
        if k == 12:
            return 'Reflect.set(globalThis, "%s", %d)' % (n, v)
        if k == 13:
            return 'delete Object.prototype.%s' % n
        if k == 14:
            return 'Object.prototype.%s = %d' % (n, v)
        return ('Object.defineProperty(Object.prototype, "%s", {get(){ return "PG2"; }, '
                'set(x){ LOG.push("pset2" + x); }, configurable: true})') % n

    def rhs(self):
        effects = ', '.join(self.effect() for _ in range(self.r.randrange(1, 3)))
        return '(%s, %d)' % (effects, self.r.randrange(100))

    def assignment(self, cond=None):
        r = self.r
        n = r.choice(NAMES)
        m = r.choice(NAMES)
        e = self.rhs() if cond is None else '(%s ? %s : %d)' % (cond, self.rhs(), r.randrange(9))
        k = r.randrange(12)
        if k <= 3:
            return '%s = %s;' % (n, e)
        if k == 4:
            return '%s += %s;' % (n, e)
        if k == 5:
            return '%s = %s = %s;' % (n, m, e)
        if k == 6:
            return '[%s, %s] = [%s, %s];' % (n, m, e, self.rhs())
        if k == 7:
            return '({a: %s, b: %s} = {get a(){ %s; return 1; }, get b(){ %s; return 2; }});' % (
                n, m, self.effect(), self.effect())
        if k == 8:
            return 'for (%s in {p: 1, q: 2}) { %s; }' % (n, self.effect())
        if k == 9:
            return 'for (%s of [1, 2]) { %s; }' % (n, self.effect())
        if k == 10:
            return '%s++;' % n if r.random() < 0.5 else '--%s;' % n
        return '%s ??= %s;' % (n, e) if r.random() < 0.5 else '%s ||= %s;' % (n, e)

    def program(self):
        r = self.r
        out = []
        w = out.append
        names = '[' + ','.join('"%s"' % n for n in NAMES) + ']'
        w('var ES = typeof __quickjsRustEvalScript == "function" ? __quickjsRustEvalScript : std.evalScript;')
        w('var LOG = [];')
        w('function D(n){ var d = Object.getOwnPropertyDescriptor(globalThis, n); if (!d) return "none"; '
          'if ("get" in d) return "acc" + (d.configurable?"c":""); '
          'return (typeof d.value) + ":" + String(d.value).slice(0,12) + (d.writable?"w":"r") + (d.configurable?"c":""); }')
        w('function V(n){ try { return String((0,eval)(n)).slice(0,12); } catch (e) { return "!" + e.name; } }')
        w('function SNAP(t){ var s = []; for (var n of ' + names + ') s.push(n + "=" + D(n) + "/" + V(n)); '
          'print(t + ": " + s.join(" ") + " L=" + LOG.splice(0).join(",")); }')
        w('function T(t, f){ try { var r = f(); LOG.push("r" + String(r).slice(0,10)); } catch (e) { LOG.push("!" + e.name); } }')
        for n in NAMES:
            k = r.randrange(8)
            if k == 0:
                w('var %s = %d;' % (n, r.randrange(10)))
            elif k == 1:
                w('globalThis.%s = %d;' % (n, r.randrange(10)))
            elif k == 2:
                w('Object.defineProperty(globalThis, "%s", {value: %d, writable: %s, configurable: true, enumerable: true});'
                  % (n, r.randrange(10), r.choice(['true', 'false'])))
            elif k == 3:
                w('Object.defineProperty(globalThis, "%s", {get(){ LOG.push("get%s"); return 5; }, '
                  'set(v){ LOG.push("set%s" + v); }, configurable: true});' % (n, n, n))
            elif k == 4 and r.random() < 0.6:
                w('let %s = %d;' % (n, r.randrange(10)))
                self.lexicals.add(n)
            elif k == 5:
                w('(0,eval)("var %s = %d");' % (n, r.randrange(10)))
        if self.proto:
            for n in r.sample(NAMES, 2):
                k = r.randrange(3)
                if k == 0:
                    w('Object.prototype.%s = "P";' % n)
                elif k == 1:
                    w('Object.defineProperty(Object.prototype, "%s", {value: "PR", writable: false, configurable: true});' % n)
                else:
                    w('Object.defineProperty(Object.prototype, "%s", {get(){ return "PG"; }, '
                      'set(v){ LOG.push("pset" + v); }, configurable: true});' % n)
        w('SNAP("init");')
        functions = r.randrange(2, 5)
        for i in range(functions):
            strict = r.random() < 0.45
            body = []
            for j in range(r.randrange(1, 4)):
                cond = r.choice([None, 'i %% %d == %d' % (r.randrange(2, 6), r.randrange(2)),
                                 'i > %d' % r.randrange(1, 40)])
                if r.random() < 0.3:
                    body.append('T("s%d", function(){ %s });' % (j, self.assignment(cond)))
                else:
                    body.append('try { %s } catch (e) { LOG.push("!" + e.name); }' % self.assignment(cond))
            w('function F%d(i) { %s%s }' % (i, '"use strict"; ' if strict else '', ' '.join(body)))
        for j in range(r.randrange(1, 4)):
            w('try { %s } catch (e) { LOG.push("!" + e.name); }' % self.assignment())
            w('SNAP("top%d");' % j)
        iterations = r.choice([3, 12, 40, 120])
        w('for (var i = 0; i < %d; i++) {' % iterations)
        for i in range(functions):
            if r.random() < 0.8:
                w('  try { F%d(i); } catch (e) { LOG.push("F!" + e.name); }' % i)
        w('  if (i %% %d == 0) SNAP("it" + i);' % max(1, iterations // 6))
        w('}')
        w('SNAP("end");')
        for i in range(functions):
            w('try { F%d(%d); } catch (e) { LOG.push("F!" + e.name); }' % (i, r.randrange(100)))
        w('SNAP("final");')
        return '\n'.join(out) + '\n'


if __name__ == '__main__':
    sys.stdout.write(GlobalStoreGen(int(sys.argv[1])).program())
