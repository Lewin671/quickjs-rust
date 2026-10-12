#include "quickjs.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
__attribute__((noinline)) static void body(JSContext*ctx, int k, int mode, JSValue blueprint) {
 for(int j=0;j<k;j++){
  JSValue v= mode==2 ? JS_EvalFunction(ctx,JS_DupValue(ctx,blueprint)) : JS_Eval(ctx,"g()",3,"<eval>",JS_EVAL_TYPE_GLOBAL | (mode==0?JS_EVAL_FLAG_COMPILE_ONLY:0));
  if(JS_IsException(v)){fprintf(stderr,"JS exception\n");exit(1);}JS_FreeValue(ctx,v);
 }
}
int main(int argc,char**argv){
 if(argc!=5)return 2;int mode=atoi(argv[1]),c=atoi(argv[2]),k=atoi(argv[3]),n=atoi(argv[4]);
 JSRuntime*rt=JS_NewRuntime();JSContext*ctx=JS_NewContext(rt);
 JSValue init=JS_Eval(ctx,"function g(){return 1;}",23,"<init>",JS_EVAL_TYPE_GLOBAL);JS_FreeValue(ctx,init);
 JSValue blueprint=JS_Eval(ctx,"g()",3,"<eval>",JS_EVAL_TYPE_GLOBAL|JS_EVAL_FLAG_COMPILE_ONLY);
 for(int i=0;i<n;i++)for(int j=0;j<c;j++)body(ctx,k,mode,blueprint);
 JS_FreeValue(ctx,blueprint);JS_FreeContext(ctx);JS_FreeRuntime(rt);puts("OK");return 0;
}
