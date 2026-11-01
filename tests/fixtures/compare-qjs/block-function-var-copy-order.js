(function(){var f,before,after;function read(){return typeof f;}
try{throw 0;function f(){}}catch(e){}var skipped=read();
{before=read();function f(){return 7;}after=read();}
return [skipped,before,after,f()].join(':');})()
