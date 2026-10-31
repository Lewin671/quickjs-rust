use crate::{Value, eval};

#[test]
fn date_methods_reject_fabricated_and_inherited_receivers() {
    assert_eq!(
        eval(
            "let real=new Date(7);let fake={'\\0DateValue':9};let inherited=Object.create(real);let rejected=0;for(let receiver of [fake,inherited,{}]){try{Date.prototype.getTime.call(receiver);}catch(e){if(e instanceof TypeError)rejected++;}}rejected===3 && real.getTime()===7;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn ordinary_nul_properties_do_not_change_date_time_or_brand() {
    assert_eq!(
        eval(
            "let date=new Date(11);let empty=Reflect.ownKeys(date).length===0;date['\\0DateValue']=99;empty && date.getTime()===11 && date['\\0DateValue']===99 && Object.getOwnPropertyNames(date).includes('\\0DateValue');"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn date_mutation_survives_freezing_and_private_field_metadata() {
    assert_eq!(
        eval(
            "let date=Object.freeze(new Date(3));date.setTime(5);class D extends Date{#x=7;read(){return this.#x+this.getTime();}}let child=new D(9);child.setTime(10);date.getTime()===5 && child.read()===17 && Object.isFrozen(date);"
        ),
        Ok(Value::Boolean(true))
    );
}
