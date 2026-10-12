/* Normal release reference assay. Scope binding/destruction and execution use NG's
 * original js_closure and JS_Call, no timer or counter hook in the engine. */
#include "quickjs.c"
#include <stdio.h>
static int mode,copies,steps;
static JSValue assay(JSContext*ctx,JSValueConst thisv,int argc,JSValueConst*argv){
 int n=0;JS_ToInt32(ctx,&n,argv[0]);JSStackFrame*saved=ctx->rt->current_stack_frame,*sf=saved;
 while(sf && (!JS_IsObject(sf->cur_func)||!js_class_has_bytecode(JS_VALUE_GET_OBJ(sf->cur_func)->class_id)))sf=sf->prev_frame;
 if(!sf)return JS_EXCEPTION;ctx->rt->current_stack_frame=sf;
 JSValue blueprint=__JS_EvalInternal(ctx,JS_UNDEFINED,"g()",3,"<input>",1,JS_EVAL_TYPE_DIRECT|JS_EVAL_FLAG_COMPILE_ONLY,0);
 if(JS_IsException(blueprint))return blueprint;
 JSObject*host=JS_VALUE_GET_OBJ(sf->cur_func);JSVarRef**refs=host->u.func.var_refs;
 JSValue bound=js_closure(ctx,JS_DupValue(ctx,blueprint),refs,sf);
 if(JS_IsException(bound))return bound;
 for(int i=0;i<n;i++)for(int j=0;j<copies;j++)for(int k=0;k<steps;k++){
  JSValue fn=mode==1?JS_DupValue(ctx,bound):js_closure(ctx,JS_DupValue(ctx,blueprint),refs,sf);
  if(JS_IsException(fn))return fn;
  if(mode!=0){JSValue v=JS_Call(ctx,fn,JS_UNDEFINED,0,NULL);if(JS_IsException(v))return v;JS_FreeValue(ctx,v);}
  JS_FreeValue(ctx,fn);
 }
 JS_FreeValue(ctx,bound);JS_FreeValue(ctx,blueprint);ctx->rt->current_stack_frame=saved;return JS_NewInt32(ctx,1);
}
int main(int argc,char**argv){
 if(argc!=6)return 2;mode=atoi(argv[1]);copies=atoi(argv[2]);steps=atoi(argv[3]);int n=atoi(argv[4]),scope=atoi(argv[5]);
 JSRuntime*rt=JS_NewRuntime();JSContext*ctx=JS_NewContext(rt);JSValue global=JS_GetGlobalObject(ctx);
 JS_SetPropertyStr(ctx,global,"assay",JS_NewCFunction(ctx,assay,"assay",1));JS_FreeValue(ctx,global);
 char src[16384];size_t pos=0;pos+=sprintf(src+pos,"function parent(n){var ");for(int i=0;i<scope;i++)pos+=sprintf(src+pos,"a%d,",i);
 pos+=sprintf(src+pos,"z;function g(){return 1;}if(n<0)eval('g()');return assay(n);}parent(%d);",n);
 JSValue v=JS_Eval(ctx,src,pos,"<host>",JS_EVAL_TYPE_GLOBAL);if(JS_IsException(v)){JSValue e=JS_GetException(ctx);const char*t=JS_ToCString(ctx,e);fprintf(stderr,"%s\n",t?t:"error");return 3;}
 JS_FreeValue(ctx,v);JS_FreeContext(ctx);JS_FreeRuntime(rt);puts("OK");return 0;
}
