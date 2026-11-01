(function(){
function tag(){var o={v:7,f(){return this===o?this.v:-1;}};with(o){return f`x`;}}
function optional(){var o={v:7,f(){return this===o?this.v:-1;}};with(o){return f?.();}}
function spread(){var o={v:7,f(){return this.v;}};with(o){return f?.(...[]);}}
function escaped(){var o={v:7,f(){return this.v;}},g;with(o){g=function(){return f?.();};}return g();}
function nullish(){var n=0;function call(){with({f:null}){return f?.(n++);}}var r=call();return typeof r+':'+n;}
function mutated(){var o={v:7,f(){return this.v;}};with(o){return f?.(delete o.f);}}
return [tag(),optional(),spread(),escaped(),nullish(),mutated()].join(':');})()
