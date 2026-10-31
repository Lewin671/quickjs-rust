(() => {
  let readOuter, readInner, InnerCtor;
  class Outer { #p;
    [(InnerCtor = class Inner extends (readOuter = obj => #p in obj, Object) {
      #p; [(readInner = obj => #p in obj, 'x')]() {}
    }, 'm')]() {}
  }
  const outer = new Outer(), inner = new InnerCtor();
  return [readOuter(outer), readOuter(inner), readInner(outer), readInner(inner)].join(':');
})()
