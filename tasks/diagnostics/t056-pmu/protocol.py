"""Lead-only counter capture, diagnostic; no acceptance or attempt decision."""
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
ALIGNED='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
CANDIDATE='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
HASHES={'base':'b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e','aligned':'b0b84489a3f6fd673b06dc4bdc71c8f176387a231ca19fe1da5ea48ca4ee0fd5','candidate':'3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f'}
CASES=('broad/global_read','broad/local_read')
COUNT=23068672
BLOCKS=2
ROLES=('base','aligned','candidate','baseaa')
TRACE_SECONDS=5
BUNDLE_SECONDS=30
COLLECT_SECONDS=1080
BODY_SECONDS=1140
JOB_SECONDS=1200
ENV_RESET=('MallocNanoZone','MallocStackLogging','MallocStackLoggingNoCompact','QJS_TL_TRACE','QJS_ARCH_PAD_MODE','QJS_DIAGNOSTIC_BANK_POLICY','QJS_DIAGNOSTIC_BANK_TRACE','T056_INITIAL_STACK_PAD','T056_STACK_COOKIE')
def order(values,block,phase=0):
 values=list(values);shift=(block+phase)%len(values);result=values[shift:]+values[:shift]
 return result[::-1] if block%2 else result
