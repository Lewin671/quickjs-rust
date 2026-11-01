class C { #p; [(#p in {}, eval('1'))]() {} }
const direct = typeof arguments;
const fromEval = eval('typeof arguments');
const fromArrow = (() => {
  class D { #p; [(#p in {}, eval('1'))]() {} }
  return typeof arguments;
})();
class Outer { static { class D { #p; [(#p in {}, eval('1'))]() {} } } }

print([direct, fromEval, fromArrow, typeof arguments].join(':'));
class Receiver { #p; [(#p in {}, String(this))]() {} }
class ArrowReceiver { #p; [(#p in {}, (() => String(this))())]() {} }
print(typeof new Receiver().undefined + ':' + typeof new ArrowReceiver().undefined);
