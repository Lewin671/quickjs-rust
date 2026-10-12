/* Standalone reference front-end assay, no counters/instrumentation and no source edits.
 * Uses the pinned NG implementation compiled with its release recipe. */
#include "quickjs.c"
#include <stdio.h>
static JSValue parse_phase(JSContext*ctx,JSFunctionBytecode*parent,int mode){
 JSParseState state,*s=&state;js_parse_init(ctx,s,"g()",3,"<input>",1);
 JSFunctionDef*fd=js_new_function_def(ctx,NULL,true,false,"<input>",1,1);
 if(!fd) return JS_EXCEPTION;s->cur_func=fd;fd->eval_type=JS_EVAL_TYPE_DIRECT;
 fd->has_this_binding=false;fd->is_strict_mode=parent->is_strict_mode;
 fd->new_target_allowed=parent->new_target_allowed;fd->super_call_allowed=parent->super_call_allowed;
 fd->super_allowed=parent->super_allowed;fd->arguments_allowed=parent->arguments_allowed;
 fd->func_name=JS_DupAtom(ctx,JS_ATOM__eval_);
 if(add_closure_variables(ctx,fd,parent,0)<0)goto fail;
 if(mode==0){ /* context construction only */ js_free_function_def(ctx,fd);return JS_UNDEFINED; }
 if(mode==1){ /* lexer, using the same context */
  do {if(next_token(s)<0)goto fail;}while(s->token.val!=TOK_EOF);
  free_token(s,&s->token);js_free_function_def(ctx,fd);return JS_UNDEFINED;
 }
 push_scope(s);fd->body_scope=fd->scope_level;s->is_module=false;s->allow_html_comments=true;
 if(js_parse_program(s)<0)goto fail;
 if(mode==2){js_free_function_def(ctx,fd);return JS_UNDEFINED;}
 return js_create_function(ctx,fd);
fail:free_token(s,&s->token);js_free_function_def(ctx,fd);return JS_EXCEPTION;
}
__attribute__((noinline)) static void body(JSContext*ctx,JSFunctionBytecode*parent,int mode,int k){
 for(int j=0;j<k;j++){JSValue v=parse_phase(ctx,parent,mode);if(JS_IsException(v)){JSValue e=JS_GetException(ctx);const char*t=JS_ToCString(ctx,e);fprintf(stderr,"%s\n",t?t:"error");exit(1);}JS_FreeValue(ctx,v);}
}
int main(int argc,char**argv){
 if(argc!=6)return 2;int mode=atoi(argv[1]),c=atoi(argv[2]),k=atoi(argv[3]),n=atoi(argv[4]),scope=atoi(argv[5]);
 JSRuntime*rt=JS_NewRuntime();JSContext*ctx=JS_NewContext(rt);char source[16384];size_t pos=0;
 pos+=sprintf(source+pos,"function parent(){var ");for(int i=0;i<scope;i++)pos+=sprintf(source+pos,"a%d,",i);
 pos+=sprintf(source+pos,"z;function g(){return 1;}return eval('g()');}");
 JSValue init=JS_Eval(ctx,source,pos,"<host>",JS_EVAL_TYPE_GLOBAL);if(JS_IsException(init))return 3;JS_FreeValue(ctx,init);
 JSValue global=JS_GetGlobalObject(ctx);JSValue host=JS_GetPropertyStr(ctx,global,"parent");JS_FreeValue(ctx,global);
 JSFunctionBytecode*parent=JS_VALUE_GET_OBJ(host)->u.func.function_bytecode;
 for(int i=0;i<n;i++)for(int j=0;j<c;j++)body(ctx,parent,mode,k);
 JS_FreeValue(ctx,host);JS_FreeContext(ctx);JS_FreeRuntime(rt);puts("OK");return 0;
}
