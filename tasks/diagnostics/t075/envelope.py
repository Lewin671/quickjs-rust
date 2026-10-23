"""Reapply T068 taxonomy v2 to the exact-PC trees, resolving original nm names.

Frozen ordered classifier from T068's 20261023T044006Z audit; its human rule
order is tasks/archive/T068-profile-evidence.md. This is an implementation-owner
partition, separate from summarize.py's inlined-instruction semantic partition.
Usage: envelope.py PROFILE_OUT OUTPUT_JSON
"""
import bisect
from collections import Counter
import functools
import json
from pathlib import Path
import re
import sys
from analyze import symbols, owner, executor

@functools.lru_cache(None)
def category(name, role):
    s = name.lower()
    if role == "ng":
        if re.search(r"js_(?:malloc|realloc|free)(?:_rt)?$|malloc|calloc|realloc|nano|tiny_|small_|large_|szone|madvise|munmap|mmap|^free$", s): return "allocation"
        if re.search(r"js_free_?value|js_dupvalue|free_zero_refcount|js_free_shape|js_free_object|free_object|free_property|free_shape", s): return "value-rc"
        if re.search(r"js_run_?gc|js_freecontext|js_freeruntime|gc_decref|gc_scan|gc_free|gc_mark|js_mark|mark_children", s): return "teardown"
        if re.search(r"js_atomize|js_newatom", s): return "strings"
        if re.search(r"js_parse|js_compile|next_token|json_parse|js_evalinternal|js_evaluate|js_newcontext|js_newruntime|js_addintrinsic|js_parse_init|skip_shebang", s):
            return "builtin-JSON" if "json" in s else "parse-compile"
        if re.search(r"lre_|js_regexp|re_exec|regexp_", s): return "regexp"
        for family in ["array", "object", "string", "json", "date", "math"]:
            if re.search(r"js_" + family + r"_", s): return "builtin-" + family.title().replace("Json", "JSON")
        if re.search(r"js_getpropertyuint|js_setpropertyuint|js_typedarray|typed_array|fast_array_element", s): return "elements"
        if re.search(r"js_getpropertyvalue|js_setpropertyvalue|js_getpropertyinternal|js_setpropertyinternal|js_getownproperty", s): return "named-other"
        if re.search(r"get_own_property|find_own_property|find_property|add_property|resize_properties|js_defineproperty|js_deleteproperty|convert_fast_array|find_prop|shape_hash|shape_property|js_newobjectfromshape|js_createproperty|js_new_shape|js_clone_shape", s): return "property-storage"
        if re.search(r"closure|var_ref|varref|close_lexical|close_var", s): return "environments"
        if re.search(r"js_call|js_invoke|js_function_call|js_function_apply|js_bound_function|js_create_arguments|js_build_arguments", s):
            return "loop" if "js_callinternal" in s else "calls"
        if re.search(r"js_throw|js_exception|build_backtrace", s): return "exceptions"
        if re.search(r"concat_string|js_concat|js_substring|js_newstring|js_string|js_to(?:c)?string|string_compare|js_toatom|js_atomtostring|memcmp", s): return "strings"
        if re.search(r"js_(?:eq|strict_eq|loose_eq|relational_slow|binary_arith_slow|to(?:primitive|bool|number|float|int|uint)|add|sub|mul|div|mod|compare)|bf_", s): return "loop"
        if s.startswith("js_"): return "builtin-other"
        return None
    if re.search(r"qjs::allocator|size_class_allocator|sizeclass|chunk.*alloc|malloc|calloc|realloc|__rust_alloc|__rust_dealloc|nanov2|tiny_|small_|large_|szone|madvise|munmap|mmap", s): return "allocation"
    if re.search(r"free$|free_small|free_large|free_tiny", s): return "allocation"
    if re.search(r"named_property_cache|cachedvalue|prototype_slot_cache|enumerate_keys_cache", s): return "named-cache"
    if re.search(r"propertystorage|propertylayout|dynamic_storage|ordered.*property|shaped.*slot|own_data_property|own_property_slot|value::object.*(?:property|layout)", s): return "property-storage"
    if re.search(r"qjs_parser|qjs_lexer|bytecode::compiler|bytecode::compile|compile_script|compile_function|script_entry|context::context::new|realm.*new|initialize_realm|install_global", s): return "parse-compile"
    if re.search(r"drop_in_place::<.*(?:context::context|realm::)|context::.*drop|realm::.*drop|value::teardown", s): return "teardown"
    if re.search(r"callenv|framebindings|dynamicbindings|function_env|apply_env|current_env|callee_env|frame_call_env|refresh_realm_backed|load_local_slow|store_local_slow|local_slot|insert_frame_cell", s): return "environments"
    if re.search(r"new_with_globals|with_frame_bytecode|eval_function_bytecode|run_prepared_bytecode_call|pop_arguments|initial_slots|drop_in_place::<.*(?:framestate|framestack|bytecode::vm::vm)", s): return "calls"
    if re.search(r"(?:clone|drop_in_place|drop_slow|::drop|::weak::upgrade|::weak::new|compact_fn::execute::store_owned)", s): return "value-rc"
    if re.search(r"regexp|regex_|regexp_", s): return "regexp"
    for family in ["array", "object", "string", "json", "date", "math"]:
        if re.search(r"qjs_runtime::" + family + r"::", s):
            return "builtin-" + family.title().replace("Json", "JSON")
    if re.search(r"get_prop_index|set_prop_index|get_element|set_element|array_index|dense_(?:read|write)|typed_array|value::array|property::array|compact_fn::array", s): return "elements"
    if re.search(r"get_prop_named|set_prop_named|op_get_prop|op_set_prop|get_named_prop|set_named_prop|qjs_runtime::property::|slot_reads|property_value|with_prototype|object_literal|objectliteralshape|define_own_property|prototype_receiver_named|try_cached_get_string|try_direct_get_string|::own_property|literal.*slot|with_literal_properties|::own_data_slot", s): return "named-other"
    if re.search(r"callenv|frameenvironment|environment|upvalue|binding|capture|lexical|closure|vm_call_env|call_context", s) and "{{closure}}" not in s: return "environments"
    if "native::call_native_family" in s: return "builtin-other"
    if re.search(r"vm_call|function::call|function::construct|function::.*prepare|call_frame|frame_stack|frame_state|vm_frame_init|prepare_bytecode|call_resolved|call_from_activation|call_callee|::vm::.*(?:new|direct_call)|native::call_|wide::activation::inline_entry|exits::exit_to_interpreter", s): return "calls"
    if re.search(r"jsstring|qjs_unicode|wtf16|qjs_runtime::string|str::|string::string|str::pattern|str::iter", s): return "strings"
    if re.search(r"exception|throw|completion|unwind", s) and "run_completion" not in s: return "exceptions"
    if re.search(r"typed_loop|compact_fn|run_current_activation|vm_numeric|vm_loop|fast_number|value::number|value::coerc|to_number|bytecode::vm::vm::run|run_general_op|run_rare_op|op_load_local|op_unary|eval_binary_without_env|typeof_value", s): return "loop"
    if re.search(r"native_|qjs_runtime::(?:map|set|symbol|number|boolean|bigint|reflect|function::prototype|iterator|promise|error|proxy|intl)::", s): return "builtin-other"
    return None


def main():
    root, output = map(Path, sys.argv[1:])
    table = symbols(root / 'canonical-nm.txt')
    programs = {}
    visible = {}
    rows = []
    for invocation in json.loads((root / 'profiles.json').read_text()):
        if invocation['role'] != 'canonical':
            continue
        text = (root / invocation['raw_path']).read_text()
        load = int(re.search(r'Load Address:\s+(0x[0-9a-f]+)',text)[1],16)
        nodes,stack = [],[]
        for line in text.split('Call graph:\n',1)[1].splitlines():
            match = re.match(r'^([ +|:!]*)(\d+) (.+)$',line)
            if not match:
                if nodes:break
                continue
            depth,count,rest = len(match[1]),int(match[2]),match[3]
            if nodes and depth <= nodes[0]['depth']:break
            while stack and nodes[stack[-1]]['depth'] >= depth:stack.pop()
            name = rest.split('  (in ')[0]
            if '(in canonical-stripped)' in rest:
                assert '...' not in rest
                address = int(rest.rsplit('[',1)[1].split(']')[0],16)-load+0x100000000
                name = owner(table,address)[1]
            node = {'depth':depth,'self':count,'name':name,'parent':stack[-1] if stack else None}
            if stack:nodes[stack[-1]]['self']-=count
            stack.append(len(nodes));nodes.append(node)
        assert all(n['self']>=0 for n in nodes)
        total=sum(n['self'] for n in nodes)
        categories,other_loop=Counter(),Counter()
        for i,node in enumerate(nodes):
            at=i;cat=None
            while cat is None:
                cat=category(nodes[at]['name'],'rust')
                at=nodes[at]['parent']
                if at is None:break
            cat=cat or 'other'
            categories[cat]+=node['self']
            if cat=='loop' and executor(node['name']) is None:other_loop[node['name']]+=node['self']
        case=invocation['id']
        if case not in programs:programs[case]={'total':0,'categories':Counter()};visible[case]=Counter()
        programs[case]['total']+=total;programs[case]['categories'].update(categories);visible[case].update(other_loop)
        rows.append({'id':case,'repeat':invocation['repeat'],'total':total,'categories':dict(categories)})
    cats=sorted({k for p in programs.values() for k in p['categories']})
    mean={k:sum(100*p['categories'][k]/p['total'] for p in programs.values())/len(programs) for k in cats}
    assert abs(sum(mean.values())-100)<1e-8
    names={k for p in visible.values() for k in p}
    other_mean={k:sum(100*visible[c][k]/programs[c]['total'] for c in programs)/len(programs) for k in names}
    output.write_text(json.dumps({'mean':mean,'profiles':rows,'other_loop_symbols':dict(sorted(other_mean.items(),key=lambda p:(-p[1],p[0])))},indent=2)+'\n')
    print(json.dumps(mean,indent=2))
    print('other loop',sum(other_mean.values()))


if __name__=='__main__':
    main()
