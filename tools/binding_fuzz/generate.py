#!/usr/bin/env python3
"""Seeded random program generator for binding-semantics differential fuzzing.

Adapted from reviewer A's gen.py/gen2.py, supplied with T077 round 3.
The simple_parameters profile preserves gen2.py's RNG consumption.
usage: python3 -m tools.binding_fuzz.generate SEED > prog.js
Programs print one 'label: value' line per observation; errors print the
constructor name only.  Var-like names and lexical names are kept disjoint so
most programs are free of early SyntaxErrors.
"""
import json
import random
import sys

VARN = ['a', 'b', 'c']          # var / params / catch params / function-scope
LEXN = ['x', 'y', 'z']          # let / const / class-free lexical names
ALLN = VARN + LEXN + ['u']      # 'u' is never declared (except by eval/with)

PRELUDE = r'''
function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
function __e(l, e) { print(l + ': caught ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e))); }
var fs = [];
var __log = [];
function __px(t) { return new Proxy(t, { has: function (o, k) { if (typeof k === 'string') __log.push('h' + k); return k in o; }, get: function (o, k, r) { if (typeof k === 'string') __log.push('g' + k); return Reflect.get(o, k, r); }, set: function (o, k, v, r) { if (typeof k === 'string') __log.push('s' + k); return Reflect.set(o, k, v, r); } }); }
'''

EPILOGUE = r'''
for (var __i = 0; __i < fs.length; __i++) { __o('fs' + __i, fs[__i]); }
__o('log', function () { return __log.join(' '); });
'''


class Ctx:
    def __init__(self, strict=False, fn=False, gen=False, asyn=False, depth=0, lex=None, loop=False, in_eval=False, arrow=False, cls=False):
        self.strict = strict
        self.fn = fn          # inside a non-arrow function (arguments available)
        self.gen = gen
        self.asyn = asyn
        self.depth = depth
        self.lex = lex if lex is not None else set()   # lexical names declared in current block
        self.loop = loop
        self.in_eval = in_eval
        self.arrow = arrow
        self.cls = cls

    def child(self, **kw):
        c = Ctx(self.strict, self.fn, self.gen, self.asyn, self.depth + 1, set(), self.loop, self.in_eval, self.arrow, self.cls)
        for k, v in kw.items():
            setattr(c, k, v)
        return c


class Gen:
    def __init__(self, seed, simple_parameters=False):
        self.simple_parameters = simple_parameters
        self.r = random.Random(seed)
        self.n = 0
        self.fnames = []

    def lab(self, p):
        self.n += 1
        return '%s%d' % (p, self.n)

    def pick(self, xs):
        return self.r.choice(xs)

    # ---------------- expressions ----------------
    def name(self):
        return self.pick(ALLN)

    def lit(self):
        k = self.r.randrange(4)
        if k == 0:
            return str(self.r.randrange(10))
        if k == 1:
            return json.dumps(self.pick(['s', 'q', 'w']))
        if k == 2:
            return self.pick(['true', 'null', 'undefined'])
        return str(self.r.randrange(100))

    def expr(self, ctx, d=0):
        r = self.r.randrange(100)
        if d > 2:
            r = self.r.randrange(30)
        if r < 12:
            return self.lit()
        if r < 34:
            return self.name()
        if r < 42:
            return 'typeof ' + self.name()
        if r < 50:
            return '(%s + %s)' % (self.expr(ctx, d + 1), self.expr(ctx, d + 1))
        if r < 55:
            return '[%s, %s]' % (self.expr(ctx, d + 1), self.expr(ctx, d + 1))
        if r < 60:
            return '(%s = %s)' % (self.name(), self.expr(ctx, d + 1))
        if r < 64:
            return '%s%s' % (self.name(), self.pick(['++', '--']))
        if r < 69:
            return 'eval(%s)' % json.dumps(self.expr(ctx, d + 1))
        if r < 72:
            return '(0, eval)(%s)' % json.dumps(self.pick(['typeof ' + self.name(), self.name()]))
        if r < 76 and ctx.fn:
            return self.pick(['arguments.length', 'arguments[0]', 'arguments[1]', '(arguments[0] = %s)' % self.expr(ctx, d + 1)])
        if r < 79 and self.fnames:
            return '%s(%s)' % (self.pick(self.fnames), self.expr(ctx, d + 1))
        if r < 82:
            return 'delete ' + self.name() if not ctx.strict else 'typeof ' + self.name()
        if r < 85:
            return '(function () { return %s; })()' % self.expr(ctx, d + 1)
        if r < 88:
            return '(() => %s)()' % self.expr(ctx, d + 1)
        if r < 94:
            return 'fs.length'
        if r < 97:
            return '`${%s}`' % self.expr(ctx, d + 1)
        return '(%s ? %s : %s)' % (self.expr(ctx, d + 1), self.expr(ctx, d + 1), self.expr(ctx, d + 1))

    # ---------------- statements ----------------
    def block(self, ctx, n=None):
        if n is None:
            n = self.r.randrange(1, 4 if ctx.depth < 3 else 2)
        return '\n'.join(self.stmt(ctx) for _ in range(n))

    def guarded(self, code):
        l = self.lab('E')
        return 'try { %s } catch (__x) { __e(%s, __x); }' % (code, json.dumps(l))

    def stmt(self, ctx):
        if ctx.depth > 4:
            return self.observe(ctx)
        w = [
            ('var', 8), ('let', 8), ('assign', 8), ('observe', 14), ('capture', 10), ('block', 4),
            ('for', 7), ('forof', 4), ('forin', 2), ('if', 3), ('func', 7), ('iife', 3), ('eval', 9),
            ('evalfn', 3), ('with', 5), ('delete', 3), ('gen', 3), ('trycatch', 4), ('cls', 3),
            ('nfe', 0 if self.simple_parameters else 2), ('label', 0), ('async', 2), ('switch', 2), ('argsfn', 3), ('fnctor', 1),
        ]
        tot = sum(x[1] for x in w)
        k = self.r.randrange(tot)
        for nm, wt in w:
            if k < wt:
                break
            k -= wt
        return getattr(self, 's_' + nm)(ctx)

    def observe(self, ctx):
        return '__o(%s, () => %s);' % (json.dumps(self.lab('L')), self.expr(ctx))

    def s_observe(self, ctx):
        return self.observe(ctx)

    def s_var(self, ctx):
        return self.guarded('var %s = %s;' % (self.pick(VARN), self.expr(ctx))) if self.r.random() < 0.5 else 'var %s = %s;' % (self.pick(VARN), self.lit())

    def s_let(self, ctx):
        free = [n for n in LEXN if n not in ctx.lex]
        if not free:
            return self.s_assign(ctx)
        nm = self.pick(free)
        ctx.lex.add(nm)
        kw = self.pick(['let', 'let', 'const'])
        # initializer may throw (TDZ etc.) -> binding stays uninitialized: interesting
        init = self.expr(ctx) if self.r.random() < 0.6 else self.lit()
        if self.r.random() < 0.3:
            return '%s %s = (function () { try { return %s; } catch (__q) { return "init-" + __q.constructor.name; } })();' % (kw, nm, init)
        return '%s %s = %s;' % (kw, nm, self.lit() if self.r.random() < 0.5 else init) if True else ''

    def s_assign(self, ctx):
        op = self.pick(['=', '+=', '=', '||=', '??='])
        return self.guarded('%s %s %s;' % (self.name(), op, self.expr(ctx)))

    def s_capture(self, ctx):
        k = self.r.randrange(5)
        if k == 0:
            return 'fs.push(() => %s);' % self.expr(ctx)
        if k == 1:
            return 'fs.push(function () { return %s; });' % self.expr(ctx.child(fn=True, gen=False, asyn=False, arrow=False))
        if k == 2:
            return 'fs.push(() => { %s = %s; return "w"; });' % (self.name(), self.expr(ctx))
        if k == 3:
            return 'fs.push(() => eval(%s));' % json.dumps(self.expr(ctx))
        return 'fs.push(() => [typeof %s, typeof %s]);' % (self.name(), self.name())

    def s_block(self, ctx):
        return '{\n%s\n}' % self.block(ctx.child())

    def s_for(self, ctx):
        kw = self.pick(['let', 'var', 'let'])
        iv = 'i' if kw == 'let' else 'iv'
        body = self.block(ctx.child(loop=True))
        if self.r.random() < 0.3:
            return 'for (%s %s = 0; %s < 3; fs.push(() => %s), %s++) {\n%s\n}' % (kw, iv, iv, iv, iv, body)
        return 'for (%s %s = 0; %s < 3; %s++) {\n%s\n}' % (kw, iv, iv, iv, body)

    def s_forof(self, ctx):
        kw = self.pick(['let', 'const', 'var'])
        kn = 'k' if kw != 'var' else 'kv'
        return 'for (%s %s of [%s, %s]) {\nfs.push(() => %s);\n%s\n}' % (kw, kn, self.lit(), self.lit(), kn, self.block(ctx.child(loop=True)))

    def s_forin(self, ctx):
        kw = self.pick(['let', 'const', 'var'])
        kn = 'k' if kw != 'var' else 'kv'
        return 'for (%s %s in {p: 1, q: 2}) {\nfs.push(() => %s);\n%s\n}' % (kw, kn, kn, self.block(ctx.child(loop=True)))

    def s_if(self, ctx):
        return 'if (%s) {\n%s\n} else {\n%s\n}' % (self.expr(ctx), self.block(ctx.child()), self.block(ctx.child()))

    def params(self):
        k = self.r.randrange(6)
        if self.simple_parameters:
            k = 5
        ps = self.r.sample(VARN, self.r.randrange(0, 3))
        if k == 0 and ps:
            ps[-1] = '%s = %s' % (ps[-1], self.pick(['1', '() => %s' % self.pick(ALLN), self.pick(ALLN), 'arguments.length']))
        if k == 1 and ps:
            ps[-1] = '...' + ps[-1]
        return ', '.join(ps), (k in (0, 1))

    def s_func(self, ctx):
        f = 'f%d' % self.n
        self.n += 1
        ps, nonsimple = self.params()
        strict = ctx.strict or (self.r.random() < 0.25 and not nonsimple)
        inner = ctx.child(fn=True, strict=strict, gen=False, asyn=False, arrow=False)
        body = ('"use strict";\n' if strict and not ctx.strict else '') + self.block(inner)
        ret = 'return [%s];' % ', '.join(self.expr(inner) for _ in range(2))
        decl = 'function %s(%s) {\n%s\n%s\n}' % (f, ps, body, ret)
        self.fnames.append(f)
        args = ', '.join(self.expr(ctx) for _ in range(self.r.randrange(0, 3)))
        return '%s\n__o(%s, () => %s(%s));' % (decl, json.dumps(self.lab('C')), f, args)

    def s_argsfn(self, ctx):
        f = 'af%d' % self.n
        self.n += 1
        strict = ctx.strict or self.r.random() < 0.3
        ps = self.r.sample(VARN, self.r.randrange(1, 3))
        inner = ctx.child(fn=True, strict=strict, gen=False, asyn=False)
        ops = []
        for _ in range(self.r.randrange(2, 5)):
            p = self.pick(ps)
            ops.append(self.pick([
                '%s = %s;' % (p, self.expr(inner)),
                'arguments[%d] = %s;' % (self.r.randrange(3), self.expr(inner)),
                'fs.push(() => [%s, arguments[0], arguments.length]);' % p,
                'fs.push(function () { return arguments.length; });',
                '__o(%s, () => [%s, arguments[0], arguments[1]]);' % (json.dumps(self.lab('A')), p),
                'eval(%s);' % json.dumps(self.pick(['%s = 7' % p, 'arguments[0] = 8', 'var %s = 9' % p])),
                ('delete arguments[0];' if not strict else 'void 0;'),
            ]))
        body = ('"use strict";\n' if strict and not ctx.strict else '') + '\n'.join(ops)
        call = ', '.join(self.lit() for _ in range(self.r.randrange(0, 4)))
        return 'function %s(%s) {\n%s\nreturn arguments;\n}\n__o(%s, () => %s(%s));' % (f, ', '.join(ps), body, json.dumps(self.lab('R')), f, call)

    def s_iife(self, ctx):
        if self.r.random() < 0.5:
            return '(() => {\n%s\n})();' % self.block(ctx.child(arrow=True))
        return '(function () {\n%s\n})();' % self.block(ctx.child(fn=True, gen=False, asyn=False))

    def evalcode(self, ctx):
        inner = ctx.child(in_eval=True)
        k = self.r.randrange(4)
        parts = []
        if k == 0:
            parts.append('var %s = %s;' % (self.pick(VARN + ['u']), self.lit()))
        elif k == 1:
            parts.append('function %s() { return %s; }' % (self.pick(VARN + ['u']), self.expr(inner)))
        elif k == 2:
            parts.append('let %s = %s;' % (self.pick(LEXN), self.lit()))
        if self.r.random() < 0.2:
            parts.insert(0, '"use strict";')
        parts.append(self.block(inner, self.r.randrange(1, 3)))
        return '\n'.join(parts)

    def s_eval(self, ctx):
        code = self.evalcode(ctx)
        if self.r.random() < 0.2:
            return self.guarded('(0, eval)(%s);' % json.dumps(code))
        return self.guarded('eval(%s);' % json.dumps(code))

    def s_evalfn(self, ctx):
        return self.guarded('fs.push(eval(%s));' % json.dumps('(function () { return %s; })' % self.expr(ctx)))

    def s_with(self, ctx):
        if ctx.strict:
            return self.observe(ctx)
        k = self.r.randrange(4)
        props = self.r.sample(ALLN, self.r.randrange(0, 3))
        obj = '{%s}' % ', '.join('%s: %s' % (p, self.lit()) for p in props)
        if k == 1:
            obj = '__px(%s)' % obj
        elif k == 2 and props:
            obj = '(function () { var o = %s; o[Symbol.unscopables] = {%s: true}; return o; })()' % (obj, self.pick(props))
        elif k == 3:
            obj = 'Object.create(%s)' % obj
        return self.guarded('with (%s) {\n%s\n}' % (obj, self.block(ctx.child())))

    def s_delete(self, ctx):
        if ctx.strict:
            return self.observe(ctx)
        return '__o(%s, () => [delete %s, typeof %s]);' % (json.dumps(self.lab('D')), self.name(), self.name())

    def s_gen(self, ctx):
        g = 'g%d' % self.n
        self.n += 1
        ps, _ = self.params()
        inner = ctx.child(fn=True, gen=True, asyn=False, strict=ctx.strict)
        body = '\n'.join('%s\nyield %s;' % (self.block(inner, 1), self.expr(inner)) for _ in range(2))
        it = 'it%d' % self.n
        out = ['function* %s(%s) {\n%s\n}' % (g, ps, body), 'var %s = %s(%s);' % (it, g, self.lit())]
        for _ in range(3):
            out.append('__o(%s, () => %s.next(%s).value);' % (json.dumps(self.lab('G')), it, self.lit()))
            out.append(self.s_assign(ctx))
        return '\n'.join(out)

    def s_trycatch(self, ctx):
        p = self.pick(VARN + ['{a: ca}', '[cb]', ''])
        catch = 'catch (%s)' % p if p else 'catch'
        return 'try {{\n%s\nthrow %s;\n}} %s {\n%s\n} finally {{\n%s\n}}' % (
            self.block(ctx.child()), self.lit(), catch, self.block(ctx.child()), self.observe(ctx))

    def s_cls(self, ctx):
        free = [n for n in LEXN if n not in ctx.lex]
        nm = self.pick(['K', 'Q'])
        if nm in ctx.lex:
            return self.observe(ctx)
        ctx.lex.add(nm)
        inner = ctx.child(fn=False, strict=True, gen=False, asyn=False, cls=True)
        ext = ' extends %s' % self.pick(['Object', '(class { m2() { return %s; } })' % self.expr(inner)]) if self.r.random() < 0.4 else ''
        ctor = ''
        if ext and self.r.random() < 0.5:
            ctor = 'constructor() { super(); this.cv = %s; }' % self.pick(['eval("typeof %s")' % self.name(), self.expr(inner), '(() => typeof %s)()' % self.name()])
        body = '%s m() { return [%s, typeof %s]; } static s = %s; f = %s; #p = %s; gp() { return this.#p; } static { fs.push(() => [typeof %s, %s]); }' % (
            ctor, self.expr(inner), nm, self.expr(inner), self.pick([self.expr(inner), 'eval(%s)' % json.dumps(self.name()), '() => this.#p']), self.expr(inner), nm, self.expr(inner))
        decl = 'class %s%s { %s }' % (nm, ext, body)
        return self.guarded('%s\n__o(%s, () => { var o = new %s(); return [o.m(), %s.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; });' % (decl, json.dumps(self.lab('K')), nm, nm))

    def s_nfe(self, ctx):
        nm = 'nf%d' % self.n
        self.n += 1
        strict = self.r.random() < 0.4
        body = ('"use strict"; ' if strict else '') + self.pick([
            'try { %s = 1; } catch (e) { return e.constructor.name; } return typeof %s;' % (nm, nm),
            'eval("%s = 2"); return typeof %s;' % (nm, nm),
            'var g = () => { %s = 3; }; try { g(); } catch (e) { return e.constructor.name; } return typeof %s;' % (nm, nm),
            'return [typeof %s, %s];' % (nm, self.expr(ctx.child(fn=True, strict=strict))),
        ])
        return '__o(%s, function () { return (function %s() { %s })(); });' % (json.dumps(self.lab('N')), nm, body)

    def s_label(self, ctx):
        lb = self.lab('lbl')
        return '%s: for (let i = 0; i < 2; i++) {\nfor (let j = 0; j < 2; j++) {\nfs.push(() => [i, j, %s]);\nif (%s) continue %s;\n%s\n}\n}' % (
            lb, self.expr(ctx), self.pick(['j === 0', 'i === 1', 'false']), lb, self.block(ctx.child(loop=True), 1))

    def s_async(self, ctx):
        inner = ctx.child(fn=True, asyn=True, gen=False)
        l = self.lab('P')
        return '(async function () {\n%s\nawait 0;\n%s\nreturn %s;\n})().then((v) => print(%s + ": " + __S(v)), (e) => print(%s + ": rej " + (e && e.constructor && e.constructor.name)));' % (
            self.block(inner, 1), self.block(inner, 1), self.expr(inner), json.dumps(l), json.dumps(l))

    def s_switch(self, ctx):
        nm = self.pick(LEXN)
        if nm in ctx.lex:
            return self.observe(ctx)
        return 'switch (%s) {\ncase 0: let %s = %s;\nfs.push(() => %s);\ncase 1:\n%s\nbreak;\ndefault:\n%s\n}' % (
            self.pick(['0', '1', '2']), nm, self.lit(), nm, self.observe(ctx), self.observe(ctx))

    def s_fnctor(self, ctx):
        return '__o(%s, () => Function(%s, %s)(%s));' % (json.dumps(self.lab('F')), json.dumps(self.pick(VARN)), json.dumps('return [typeof %s, typeof %s, arguments.length];' % (self.name(), self.name())), self.lit())

    def program(self):
        top = Ctx(strict=self.r.random() < 0.12)
        body = self.block(top, self.r.randrange(6, 16))
        head = '"use strict";\n' if top.strict else ''
        # Disjoint names: make sure var names exist as globals sometimes
        return head + PRELUDE + body + '\n' + EPILOGUE


if __name__ == '__main__':
    seed = int(sys.argv[1])
    sys.stdout.write(Gen(seed).program())
