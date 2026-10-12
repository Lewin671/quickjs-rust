//! Cold-entry contract: a hot miss proves no ordinary/exotic resolution result.

use std::{collections::HashMap, rc::Rc};

use super::{get_prop_named, get_prop_named_after_hot_miss, get_prop_named_hot};
use crate::bytecode::named_property_cache::NamedPropertyCache;
use crate::function::{CallEnv, new_realm};
use crate::value::Property;
use crate::{ObjectRef, Value};

#[test]
fn a_hot_miss_still_checks_own_shadows_accessors_and_changed_prototypes() {
    let env = CallEnv::new(new_realm(HashMap::new()));
    let key: Rc<str> = Rc::from("m");
    let holder = ObjectRef::with_prototype(HashMap::new(), None);
    holder.set("m".to_owned(), Value::Number(1.0));
    let first = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()));
    let second = ObjectRef::with_prototype(HashMap::new(), Some(holder));
    let cache = NamedPropertyCache::default();
    for object in [&first, &second, &first, &second] {
        let receiver = Value::Object(object.clone());
        assert!(get_prop_named_hot(&receiver, &cache).is_none());
        assert_eq!(
            get_prop_named_after_hot_miss(&receiver, &key, &cache, &env),
            Ok(Value::Number(1.0))
        );
    }
    second.set("m".to_owned(), Value::Number(2.0));
    let receiver = Value::Object(second.clone());
    assert!(get_prop_named_hot(&receiver, &cache).is_none());
    assert_eq!(
        get_prop_named_after_hot_miss(&receiver, &key, &cache, &env),
        Ok(Value::Number(2.0))
    );
    second.define_property("m".to_owned(), Property::accessor(None, None, true, true));
    assert!(get_prop_named_hot(&receiver, &cache).is_none());
    assert_eq!(
        get_prop_named_after_hot_miss(&receiver, &key, &cache, &env),
        Ok(Value::Undefined)
    );
    second.delete_own_property("m");
    let replacement = ObjectRef::with_prototype(HashMap::new(), None);
    replacement.set("m".to_owned(), Value::Number(3.0));
    second
        .set_prototype(Some(replacement))
        .expect("ordinary extensible receiver");
    assert!(get_prop_named_hot(&receiver, &cache).is_none());
    assert_eq!(
        get_prop_named_after_hot_miss(&receiver, &key, &cache, &env),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn a_hot_miss_on_primitives_keeps_length_and_nullish_errors() {
    let env = CallEnv::new(new_realm(HashMap::new()));
    let key: Rc<str> = Rc::from("length");
    for receiver in [Value::String("a😀".into()), Value::Null, Value::Undefined] {
        let cache = NamedPropertyCache::default();
        assert!(get_prop_named_hot(&receiver, &cache).is_none());
        assert_eq!(
            get_prop_named_after_hot_miss(&receiver, &key, &cache, &env),
            get_prop_named(&receiver, &key, &cache, &env)
        );
    }
    assert_eq!(
        get_prop_named_after_hot_miss(
            &Value::String("a😀".into()),
            &key,
            &NamedPropertyCache::default(),
            &env
        ),
        Ok(Value::Number(3.0))
    );
}
