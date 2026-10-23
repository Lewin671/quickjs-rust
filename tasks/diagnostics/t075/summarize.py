"""T075 ordered semantic taxonomy and compact, reviewable evidence exporter.

Usage: summarize.py ANALYSIS_DIR PROFILE_OUT OUTPUT_DIR [QUICKJS_C]
Every executor self sample gets exactly one category. Rules describe sampled
instructions and inlined source contexts, not saved work or causal stall time.
"""
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path
import re
import sys

CATEGORIES = ['dispatch', 'transport', 'tags', 'rc', 'bounds', 'named-property',
              'indexed-access', 'bindings', 'calls', 'completion', 'typed-guards',
              'arithmetic', 'other']
WIDE = 'bytecode/compact_fn/wide/activation.rs'
TYPED = 'bytecode/typed_loop/execute.rs'


def relative(path):
    for prefix in ('/library/', '/qjs-runtime/src/', '/quickjs-ng/'):
        if prefix in path:
            return path.split(prefix, 1)[1]
    return path


def rust_category(row):
    frames = row['frames']
    if not frames:
        return 'other'
    inner = frames[0]
    file, fun, line = relative(inner['FileName']), inner['FunctionName'].replace('::<','<'), inner['Line']
    chain = ' '.join(relative(f['FileName']) + ':' + f['FunctionName'] for f in frames)
    op = row['instruction'].split()[0] if row['instruction'].split() else ''
    wide = next((f['Line'] for f in frames if relative(f['FileName']) == WIDE and re.search(r'(?:^|::)run_frames$',f['FunctionName'])), -1)
    typed = next((f['Line'] for f in frames if relative(f['FileName']) == TYPED and re.search(r'(?:^|::)execute(?:::)?<',f['FunctionName'])), -1)
    vm = next((f['Line'] for f in frames if relative(f['FileName']) == 'bytecode/vm.rs'), -1)
    # Physical RC manipulation, excluding identity/ptr_eq tests and RefCell borrows.
    if file.endswith('rc.rs') and re.search(r'inc_strong|dec_strong|inc_weak|dec_weak|clone|drop|upgrade|strong_count', fun):
        return 'rc'
    # PC range checks belong to dispatch, not operand-bounds checks.
    if any(s in chain for s in ('wide::WideOp', 'typed_loop::TypedOp', 'ir::Op')) and re.match(r'(?:core|alloc)/(?:src/)?(?:slice|ptr|vec|raw_vec)/',file):
        return 'dispatch'
    if (wide in (755, 758, 761) or typed in (237, 247, 248) or vm in (529, 536, 539)):
        return 'dispatch'
    # Only identifiable compare/conditional-branch instructions at slice checks.
    if 'slice/index.rs' in file and (op.startswith(('cmp', 'test')) or (op.startswith('j') and op not in ('jmp', 'jmpq'))):
        return 'bounds'
    # Actual replace/write and register clearing remain transport even in calls.
    if ('replace<qjs_runtime::value::Value>' in fun or 'write<qjs_runtime::value::Value>' in fun
            or ('compact_fn/execute.rs' in file and fun.rsplit('::',1)[-1] in ('store', 'store_owned') and line not in (42,))):
        return 'transport'
    if 'clone_local_value' in fun or ('compact_fn/execute.rs' in file and fun.rsplit('::',1)[-1] == 'store' and line == 42):
        return 'tags'
    if re.search(r'conversion.rs|html_dda.rs|value/access.rs|value/coerc', file) or re.search(r'boxed_truthiness|from_value|unbox|to_numeric|to_value|::number|number\(', fun):
        return 'tags'
    if 940 <= wide <= 957 or 959 <= wide <= 978 or 1099 <= wide <= 1114 or typed in (250,251,343,344,345,346,347,348,349,350):
        return 'tags'
    if 'named_property_cache' in chain or 'compact_fn/property.rs' in chain or 'value/object/slot_reads.rs' in chain:
        return 'named-property'
    if 814 <= wide <= 912 or 355 <= typed <= 451:
        return 'named-property'
    if re.search(r'guard_receiver|validate_receiver|guard_intrinsic', chain) or 527 <= typed <= 553:
        return 'typed-guards'
    if ('value/array.rs' in chain or re.search(r'dense_read|dense_write|element_read|computed_read|computed_write|missing_element', chain)
            or 282 <= typed <= 333 or 452 <= typed <= 526 or 1035 <= wide <= 1071):
        return 'indexed-access'
    if 'function/upvalue.rs' in chain or 'upvalue_cell' in chain or 776 <= wide <= 805 or 559 <= vm <= 630:
        return 'bindings'
    if re.search(r'try_trait|ops/try_trait|from_residual|result.rs', file) or 'unwind' in fun or 1145 <= wide <= 1151:
        return 'completion'
    if (1115 <= wide <= 1144 or 1152 <= wide <= 1460 or 695 <= wide <= 754
            or re.search(r'admit|inline_entry|seed_lexical|callee_|parameter_slots|reserve_register|FRAME_STORAGE|FrameStorage', chain)):
        return 'calls'
    if (0 < typed < 237 or 609 <= typed <= 632 or 'seed_registers' in chain or 'residency' in chain):
        return 'typed-guards'
    if ('fast_number_binary' in chain or 'typed_binary' in chain or 'typed_unary' in chain
            or 913 <= wide <= 936 or 979 <= wide <= 1034 or 253 <= typed <= 281 or 604 <= typed <= 608 or 633 <= typed <= 644):
        return 'arithmetic'
    if ('slice/index.rs' in file or 'vec/' in file or 'raw_vec/' in file or 762 <= wide <= 775
            or 806 <= wide <= 813 or wide in (937, 938, 1097, 1105, 1113)
            or typed in (249, 342, 352, 353) or 'clear_window' in fun):
        return 'transport'
    return 'other'


def ng_category(row, source):
    frames = row['frames']
    if not frames:
        return 'other'
    inner = frames[0]
    fun, line = inner['FunctionName'].replace('::<','<'), inner['Line']
    if fun in ('js_dup', 'JS_FreeValue', 'JS_FreeValueRT', 'js_dup_shape'):
        return 'rc'
    if 'get_u' in fun or fun == 'js_poll_interrupts':
        return 'dispatch'
    if re.search(r'find_own_property|find_hashed_shape_proto|AtomToValue|JS_GetGlobalVar|JS_SetGlobalVar|JS_DefinePropertyValue|JS_SetPropertyInternal', fun):
        return 'named-property'
    if fun in ('set_value',):
        return 'transport'
    if fun.startswith('JS_Is') or fun in ('js_number', 'js_int32', 'double_is_int32', 'js_operator_typeof'):
        return 'tags'
    if fun in ('js_check_stack_overflow', 'js_build_arguments'):
        return 'calls'
    if fun in ('close_var_refs',):
        return 'bindings'
    if fun != 'JS_CallInternal' or line == 0:
        return 'other'
    text = source[line-1] if line <= len(source) else ''
    if re.search(r'\bBREAK\b|SWITCH\(|opcode =|pc \+=|\*pc\+\+', text):
        # One-line local op + BREAK remains ambiguous rather than pure dispatch.
        if 'CASE(' not in text and 'set_value' not in text:
            return 'dispatch'
    if re.search(r'JS_IsException|goto exception|ret_val = JS_EXCEPTION', text):
        return 'completion'
    case = ''
    for previous in reversed(source[max(0,line-90):line]):
        if match := re.search(r'CASE\(OP_([a-zA-Z0-9_]+)\)', previous):
            case = match[1]
            break
    if re.match(r'get_loc|put_loc|set_loc|get_arg|put_arg|set_arg|push|drop|dup|swap|rot|perm|insert|nip', case):
        return 'transport'
    if re.match(r'get_var_ref|put_var_ref|set_var_ref|close_loc', case):
        return 'bindings'
    if re.search(r'field|global|var|private|array', case):
        return 'named-property'
    if re.search(r'call|return', case) or line < 17700:
        return 'calls'
    if re.search(r'add|sub|mul|div|mod|eq|less|greater|neg|not|shl|sar|shr|and|xor|inc|dec', case):
        return 'arithmetic'
    return 'other'


def main():
    analysis, root, out = map(Path, sys.argv[1:4])
    source = Path(sys.argv[4]).read_text().splitlines() if len(sys.argv) > 4 else []
    data = json.loads((analysis / 'self-pcs.json').read_text())
    assert len(data) == 90 and {p['repeat'] for p in data} == {0,1,2}
    assert sum(p['ambiguous_self'] for p in data) == 0
    out.mkdir(parents=True, exist_ok=True)
    totals, counts, pc_map, all_frames, frame_ids = Counter(), defaultdict(Counter), {}, [], {}
    profiles, lines = [], defaultdict(Counter)
    with (out / 'counts.csv').open('w') as stream:
        writer = csv.writer(stream,lineterminator='\n')
        writer.writerow(['program','role','repeat','pc','count'])
        for profile in data:
            role, case = profile['role'], profile['id']
            totals[role,case] += profile['total']
            rows = profile['executors']
            profiles.append({k:profile[k] for k in ('id','role','repeat','total','ambiguous_self')} | {'executor_self':sum(r['count'] for r in rows)})
            for row in rows:
                assert row['frames'] and row['debug_pc'] and row['instruction'] != '?', row
                category = ng_category(row,source) if role == 'ng' else rust_category(row)
                counts[role,case][category] += row['count']
                writer.writerow([case,role,profile['repeat'],row['pc'],row['count']])
                ids = []
                for frame in row['frames']:
                    key = (relative(frame['FileName']),frame['Line'],frame['FunctionName'])
                    if key not in frame_ids:
                        frame_ids[key] = len(all_frames)
                        all_frames.append(key)
                    ids.append(frame_ids[key])
                key = (role,row['pc'])
                value = [row['family'],category,row['instruction'],';'.join(map(str,ids)),row['debug_pc']]
                assert key not in pc_map or pc_map[key] == value
                pc_map[key] = value
    with (out / 'pc-map.csv').open('w') as stream:
        writer=csv.writer(stream,lineterminator='\n')
        writer.writerow(['role','pc','family','category','instruction','frame_ids','mapped_pc'])
        for key,value in sorted(pc_map.items()):writer.writerow([*key,*value])
    (out / 'frames.json').write_text('[\n'+',\n'.join(json.dumps(frame,separators=(',',':')) for frame in all_frames)+'\n]\n')
    (out / 'profiles.json').write_text(json.dumps(profiles,indent=2)+'\n')
    summary = {}
    for role in ('canonical','debug','ng'):
        programs = sorted(c for r,c in totals if r==role)
        mean = {cat:sum(100*counts[role,c][cat]/totals[role,c] for c in programs)/len(programs) for cat in CATEGORIES}
        denominator = sum(mean.values())
        summary[role]={'total_samples':sum(totals[role,c] for c in programs), 'main_thread_percent':mean,
            'executor_percent':{k:100*v/denominator for k,v in mean.items()}, 'executor_main_percent':denominator,
            'programs':{c:{'total':totals[role,c],'categories':dict(counts[role,c])} for c in programs}}
    (out / 'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    mechanisms = defaultdict(Counter)
    patterns = {'replace_value':'replace<qjs_runtime::value::Value>', 'clone_local':'clone_local_value',
        'clear_window':'clear_window', 'probe_hot':'probe_hot', 'shared_slot':'shared_data_slot_value',
        'upvalue_cell':'upvalue_cell', 'received_index':'cell_received_upvalue_index',
        'upvalue_borrow':'function/upvalue.rs', 'computed_read':'computed_read',
        'dense_read':'dense_read', 'element_read':'element_read'}
    for profile in data:
        if profile['role'] != 'canonical':
            continue
        for row in profile['executors']:
            chain = ' '.join(f['FunctionName']+' '+f['FileName'] for f in row['frames'])
            keys = [k for k,pattern in patterns.items() if pattern in chain]
            instruction = row['instruction']
            if re.match(r'(?:v?mov\w*)\s+(?:-?0x[0-9a-f]+)?\(%(?:rbp|rsp)\),\s*%',instruction):
                keys.append('direct_stack_load')
            if re.search(r',\s*(?:-?0x[0-9a-f]+)?\(%(?:rbp|rsp)\)',instruction) and instruction.startswith(('mov','vmov')):
                keys.append('direct_stack_store')
            for key in keys:
                mechanisms[profile['id']][key] += row['count']
    programs = summary['canonical']['programs']
    reach = {k:sum(100*mechanisms[c][k]/programs[c]['total'] for c in programs)/10
             for k in sorted({k for counts in mechanisms.values() for k in counts})}
    (out / 'mechanism-reach.json').write_text(json.dumps({'overlapping_main_thread_percent':reach,
        'program_counts':{c:dict(v) for c,v in mechanisms.items()}},indent=2)+'\n')

    print(json.dumps(summary,indent=2))


if __name__ == '__main__':
    main()
