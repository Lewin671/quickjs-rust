use std::collections::HashMap;

use crate::{ArrayRef, GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, call_function};

use super::{
    PROMISE_FULFILL_REACTION, PROMISE_HANDLER, PROMISE_JOBS, PROMISE_REACTION_ARGUMENT,
    PROMISE_REACTION_CAPABILITY, PROMISE_REJECTED, PROMISE_THEN, PROMISE_THENABLE,
    PROMISE_THENABLE_CAPABILITY, is_promise_object, reaction_is_fulfill, resolve_promise,
    resolving_function, settle_promise,
};
use crate::NativeFunction;

pub(super) fn enqueue_promise_reaction_job(
    env: &mut HashMap<String, Value>,
    reaction: &ObjectRef,
    argument: Value,
) {
    let job = ObjectRef::new(HashMap::new());
    if let Some(handler) = reaction
        .own_property(PROMISE_HANDLER)
        .map(|property| property.value)
    {
        job.define_non_enumerable(PROMISE_HANDLER.to_owned(), handler);
    }
    if let Some(capability) = reaction
        .own_property(PROMISE_REACTION_CAPABILITY)
        .map(|property| property.value)
    {
        job.define_non_enumerable(PROMISE_REACTION_CAPABILITY.to_owned(), capability);
    }
    job.define_non_enumerable(PROMISE_REACTION_ARGUMENT.to_owned(), argument);
    job.define_non_enumerable(
        PROMISE_FULFILL_REACTION.to_owned(),
        Value::Boolean(reaction_is_fulfill(reaction)),
    );
    let jobs = promise_jobs(env);
    jobs.set(jobs.len(), Value::Object(job));
}

pub(super) fn enqueue_promise_thenable_job(
    env: &mut HashMap<String, Value>,
    promise: ObjectRef,
    thenable: Value,
    then: Value,
) {
    let job = ObjectRef::new(HashMap::new());
    job.define_non_enumerable(
        PROMISE_THENABLE_CAPABILITY.to_owned(),
        Value::Object(promise),
    );
    job.define_non_enumerable(PROMISE_THENABLE.to_owned(), thenable);
    job.define_non_enumerable(PROMISE_THEN.to_owned(), then);
    let jobs = promise_jobs(env);
    jobs.set(jobs.len(), Value::Object(job));
}

pub(crate) fn drain_promise_jobs(env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    loop {
        let jobs = promise_jobs(env);
        let pending = jobs.to_vec();
        if pending.is_empty() {
            return Ok(());
        }
        jobs.replace_with(Vec::new());
        for job in pending {
            let Value::Object(job) = job else {
                continue;
            };
            if job.own_property(PROMISE_THENABLE).is_some() {
                run_promise_thenable_job(&job, env)?;
            } else {
                run_promise_reaction_job(&job, env)?;
            }
        }
    }
}

fn run_promise_reaction_job(
    job: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let capability = match job
        .own_property(PROMISE_REACTION_CAPABILITY)
        .map(|property| property.value)
    {
        Some(Value::Object(promise)) if is_promise_object(&promise) => promise,
        _ => return Ok(()),
    };
    let argument = job
        .own_property(PROMISE_REACTION_ARGUMENT)
        .map_or(Value::Undefined, |property| property.value);
    let fulfill = matches!(
        job.own_property(PROMISE_FULFILL_REACTION)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    );
    let handler = job
        .own_property(PROMISE_HANDLER)
        .map_or(Value::Undefined, |property| property.value);

    match handler {
        Value::Function(_) => {
            match call_function(handler, Value::Undefined, vec![argument], env, false) {
                Ok(value) => resolve_promise(&capability, value, env),
                Err(error) => settle_promise(
                    &capability,
                    PROMISE_REJECTED,
                    error.thrown.map_or(Value::Undefined, |value| *value),
                    env,
                ),
            }
        }
        _ if fulfill => settle_promise(&capability, super::PROMISE_FULFILLED, argument, env),
        _ => settle_promise(&capability, PROMISE_REJECTED, argument, env),
    }
    Ok(())
}

fn run_promise_thenable_job(
    job: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let capability = match job
        .own_property(PROMISE_THENABLE_CAPABILITY)
        .map(|property| property.value)
    {
        Some(Value::Object(promise)) if is_promise_object(&promise) => promise,
        _ => return Ok(()),
    };
    let thenable = job
        .own_property(PROMISE_THENABLE)
        .map_or(Value::Undefined, |property| property.value);
    let then = job
        .own_property(PROMISE_THEN)
        .map_or(Value::Undefined, |property| property.value);
    let resolve = resolving_function(
        "resolve",
        NativeFunction::PromiseResolveFunction,
        Value::Object(capability.clone()),
    );
    let reject = resolving_function(
        "reject",
        NativeFunction::PromiseRejectFunction,
        Value::Object(capability.clone()),
    );
    if let Err(error) = call_function(then, thenable, vec![resolve, reject], env, false) {
        settle_promise(
            &capability,
            PROMISE_REJECTED,
            error.thrown.map_or(Value::Undefined, |value| *value),
            env,
        );
    }
    Ok(())
}

fn promise_jobs(env: &mut HashMap<String, Value>) -> ArrayRef {
    let global_this = match env.get(GLOBAL_THIS_BINDING).cloned() {
        Some(Value::Object(global_this)) => global_this,
        _ => {
            let global_this = ObjectRef::new(HashMap::new());
            env.insert(
                GLOBAL_THIS_BINDING.to_owned(),
                Value::Object(global_this.clone()),
            );
            global_this
        }
    };
    match global_this
        .own_property(PROMISE_JOBS)
        .map(|property| property.value)
    {
        Some(Value::Array(jobs)) => jobs,
        _ => {
            let jobs = ArrayRef::new(Vec::new());
            global_this.define_non_enumerable(PROMISE_JOBS.to_owned(), Value::Array(jobs.clone()));
            jobs
        }
    }
}
