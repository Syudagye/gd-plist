use serde::{
    de::{Deserialize, DeserializeOwned},
    ser::Serialize,
};
use std::{collections::BTreeMap, fmt::Debug, fs::File, io::Cursor};

use crate::{
    stream::{private::Sealed, Event, OwnedEvent, Writer},
    Deserializer, Dictionary, Error, Integer, Serializer, Uid, Value,
};

struct VecWriter {
    events: Vec<OwnedEvent>,
}

impl VecWriter {
    pub fn new() -> VecWriter {
        VecWriter { events: Vec::new() }
    }

    pub fn into_inner(self) -> Vec<OwnedEvent> {
        self.events
    }
}

impl Writer for VecWriter {
    fn write_start_dictionary(&mut self, len: Option<u64>) -> Result<(), Error> {
        self.events.push(Event::StartDictionary(len));
        Ok(())
    }

    fn write_end_collection(&mut self) -> Result<(), Error> {
        self.events.push(Event::EndCollection);
        Ok(())
    }

    fn write_boolean(&mut self, value: bool) -> Result<(), Error> {
        self.events.push(Event::Boolean(value));
        Ok(())
    }

    fn write_integer(&mut self, value: Integer) -> Result<(), Error> {
        self.events.push(Event::Integer(value));
        Ok(())
    }

    fn write_real(&mut self, value: f64) -> Result<(), Error> {
        self.events.push(Event::Real(value));
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> Result<(), Error> {
        self.events.push(Event::String(value.to_owned().into()));
        Ok(())
    }

    fn write_uid(&mut self, value: Uid) -> Result<(), Error> {
        self.events.push(Event::Uid(value));
        Ok(())
    }
}

impl Sealed for VecWriter {}

fn new_serializer() -> Serializer<VecWriter> {
    Serializer::new(VecWriter::new())
}

fn new_deserializer(events: Vec<OwnedEvent>) -> Deserializer<Vec<Result<OwnedEvent, Error>>> {
    let result_events = events.into_iter().map(Ok).collect();
    Deserializer::new(result_events)
}

fn assert_roundtrip<T>(obj: T, comparison: Option<&[Event]>)
where
    T: Debug + DeserializeOwned + PartialEq + Serialize,
{
    let mut se = new_serializer();

    obj.serialize(&mut se).unwrap();

    let events = se.into_inner().into_inner();

    if let Some(comparison) = comparison {
        assert_eq!(&events[..], comparison);
    }

    let mut de = new_deserializer(events);

    let new_obj = T::deserialize(&mut de).unwrap();

    assert_eq!(new_obj, obj);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Animal {
    Cow,
    Dog(Dog),
    Cat {
        age: Integer,
        name: String,
        firmware: Option<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Dog {
    a: (),
    b: usize,
    c: Option<Uid>,
}

#[test]
fn cow() {
    let cow = Animal::Cow;

    let comparison = &[Event::String("Cow".into())];

    assert_roundtrip(cow, Some(comparison));
}

#[test]
fn dog() {
    let dog = Animal::Dog(Dog {
        a: (),
        b: 12,
        c: Some(Uid::new(42)),
    });

    let comparison = &[
        Event::StartDictionary(Some(1)),
        Event::String("Dog".into()),
        Event::StartDictionary(None),
        Event::String("a".into()),
        Event::String("".into()),
        Event::String("b".into()),
        Event::Integer(12.into()),
        Event::String("d".into()),
        Event::Uid(Uid::new(42)),
        Event::EndCollection,
        Event::EndCollection,
    ];

    assert_roundtrip(dog, Some(comparison));
}

#[test]
fn cat_with_firmware() {
    let cat = Animal::Cat {
        age: 12.into(),
        name: "Paws".to_owned(),
        firmware: Some(8),
    };

    let comparison = &[
        Event::StartDictionary(Some(1)),
        Event::String("Cat".into()),
        Event::StartDictionary(None),
        Event::String("age".into()),
        Event::Integer(12.into()),
        Event::String("name".into()),
        Event::String("Paws".into()),
        Event::String("firmware".into()),
        Event::Integer(8.into()),
        Event::EndCollection,
        Event::EndCollection,
    ];

    assert_roundtrip(cat, Some(comparison));
}

#[test]
fn cat_without_firmware() {
    let cat = Animal::Cat {
        age: Integer::from(-12),
        name: "Paws".to_owned(),
        firmware: None,
    };

    let comparison = &[
        Event::StartDictionary(Some(1)),
        Event::String("Cat".into()),
        Event::StartDictionary(None),
        Event::String("age".into()),
        Event::Integer(Integer::from(-12)),
        Event::String("name".into()),
        Event::String("Paws".into()),
        Event::EndCollection,
        Event::EndCollection,
    ];

    assert_roundtrip(cat, Some(comparison));
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TypeWithOptions {
    a: Option<String>,
    b: Option<Option<u32>>,
    c: Option<Box<TypeWithOptions>>,
}

#[test]
fn type_with_options() {
    let inner = TypeWithOptions {
        a: None,
        b: Some(Some(12)),
        c: None,
    };

    let obj = TypeWithOptions {
        a: Some("hello".to_owned()),
        b: Some(None),
        c: Some(Box::new(inner)),
    };

    let comparison = &[
        Event::StartDictionary(None),
        Event::String("a".into()),
        Event::String("hello".into()),
        Event::String("b".into()),
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
        Event::String("c".into()),
        Event::StartDictionary(None),
        Event::String("b".into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::Integer(12.into()),
        Event::EndCollection,
        Event::EndCollection,
        Event::EndCollection,
    ];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_some() {
    let obj = Some(12);

    let comparison = &[Event::Integer(12.into())];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_none() {
    let obj: Option<u32> = None;

    let comparison = &[];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_some_some() {
    let obj = Some(Some(12));

    let comparison = &[
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::Integer(12.into()),
        Event::EndCollection,
    ];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_some_none() {
    let obj: Option<Option<u32>> = Some(None);

    let comparison = &[
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
    ];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_dictionary_values() {
    let mut obj = BTreeMap::new();
    obj.insert("a".to_owned(), None);
    obj.insert("b".to_owned(), Some(None));
    obj.insert("c".to_owned(), Some(Some(144)));

    let comparison = &[
        Event::StartDictionary(Some(3)),
        Event::String("a".into()),
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
        Event::String("b".into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
        Event::EndCollection,
        Event::String("c".into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::Integer(144.into()),
        Event::EndCollection,
        Event::EndCollection,
        Event::EndCollection,
    ];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn option_dictionary_keys() {
    let mut obj = BTreeMap::new();
    obj.insert(None, 1);
    obj.insert(Some(None), 2);
    obj.insert(Some(Some(144)), 3);

    let comparison = &[
        Event::StartDictionary(Some(3)),
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
        Event::Integer(1.into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::StartDictionary(Some(1)),
        Event::String("None".into()),
        Event::String("".into()),
        Event::EndCollection,
        Event::EndCollection,
        Event::Integer(2.into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::StartDictionary(Some(1)),
        Event::String("Some".into()),
        Event::Integer(144.into()),
        Event::EndCollection,
        Event::EndCollection,
        Event::Integer(3.into()),
        Event::EndCollection,
    ];

    assert_roundtrip(obj, Some(comparison));
}

#[test]
fn enum_variant_types() {
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    enum Foo {
        Unit,
        Struct { v: u32, s: String },
    }

    let expected = &[Event::String("Unit".into())];
    assert_roundtrip(Foo::Unit, Some(expected));

    let expected = &[
        Event::StartDictionary(Some(1)),
        Event::String("Struct".into()),
        Event::StartDictionary(None),
        Event::String("v".into()),
        Event::Integer(42.into()),
        Event::String("s".into()),
        Event::String("bar".into()),
        Event::EndCollection,
        Event::EndCollection,
    ];
    assert_roundtrip(
        Foo::Struct {
            v: 42,
            s: "bar".into(),
        },
        Some(expected),
    );
}

#[test]
fn deserialise_old_enum_unit_variant_encoding() {
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    enum Foo {
        Bar,
        Baz,
    }

    // `plist` before v1.1 serialised unit enum variants as if they were newtype variants
    // containing an empty string.
    let events = &[
        Event::StartDictionary(Some(1)),
        Event::String("Baz".into()),
        Event::String("".into()),
        Event::EndCollection,
    ];

    let mut de = new_deserializer(events.to_vec());
    let obj = Foo::deserialize(&mut de).unwrap();

    assert_eq!(obj, Foo::Baz);
}

#[test]
fn deserialize_dictionary_xml() {
    let reader = File::open("./tests/data/xml.plist").unwrap();
    let dict: Dictionary = crate::from_reader(reader).unwrap();

    check_common_plist(&dict);

    // xml.plist has this member, but binary.plist does not.
    assert_eq!(
        dict.get("HexademicalNumber") // sic
            .unwrap()
            .as_unsigned_integer()
            .unwrap(),
        0xDEADBEEF
    );
}

// Shared checks used by the tests deserialize_dictionary_xml() and
// deserialize_dictionary_binary(), which load files with different formats
// but the same data elements.
fn check_common_plist(dict: &Dictionary) {
    // Dictionary
    //
    // There is no embedded dictionary in this plist.  See
    // deserialize_dictionary_binary_nskeyedarchiver() below for an example
    // of that.

    // Boolean elements

    assert!(dict.get("IsTrue").unwrap().as_boolean().unwrap());

    assert!(!dict.get("IsNotFalse").unwrap().as_boolean().unwrap());

    // Real

    assert_eq!(dict.get("Height").unwrap().as_real().unwrap(), 1.6);

    // Integer

    assert_eq!(
        dict.get("BiggestNumber")
            .unwrap()
            .as_unsigned_integer()
            .unwrap(),
        18446744073709551615
    );

    assert_eq!(
        dict.get("Death").unwrap().as_unsigned_integer().unwrap(),
        1564
    );

    assert_eq!(
        dict.get("SmallestNumber")
            .unwrap()
            .as_signed_integer()
            .unwrap(),
        -9223372036854775808
    );

    // String

    assert_eq!(
        dict.get("Author").unwrap().as_string().unwrap(),
        "William Shakespeare"
    );

    assert_eq!(dict.get("Blank").unwrap().as_string().unwrap(), "");

    // Uid
    //
    // No checks for Uid value type in this test. See
    // deserialize_dictionary_binary_nskeyedarchiver() below for an example
    // of that.
}

#[test]
fn dictionary_deserialize_dictionary_in_struct() {
    // Example from <https://github.com/ebarnard/rust-plist/issues/54>
    #[derive(Deserialize)]
    struct LayerinfoData {
        color: Option<String>,
        lib: Option<Dictionary>,
    }

    let lib_dict: LayerinfoData = crate::from_bytes(r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <d>
            <k>color</k>
            <s>1,0.75,0,0.7</s>
            <k>lib</k>
            <d>
            <k>com.typemytype.robofont.segmentType</k>
            <s>curve</s>
            </d>
        </d>
        </plist>
        "#.as_bytes()).unwrap();

    assert_eq!(lib_dict.color.unwrap(), "1,0.75,0,0.7");
    assert_eq!(
        lib_dict
            .lib
            .unwrap()
            .get("com.typemytype.robofont.segmentType")
            .unwrap()
            .as_string()
            .unwrap(),
        "curve"
    );
}

#[test]
fn dictionary_serialize_xml() {
    // Dictionary to be embedded in dict, below.
    let mut inner_dict = Dictionary::new();
    inner_dict.insert(
        "FirstKey".to_owned(),
        Value::String("FirstValue".to_owned()),
    );
    inner_dict.insert("SecondKey".to_owned(), Value::Real(1.234));

    // Top-level dictionary.
    let mut dict = Dictionary::new();
    dict.insert("ADictionary".to_owned(), Value::Dictionary(inner_dict));
    dict.insert("AnInteger".to_owned(), Value::Integer(Integer::from(123)));
    dict.insert("ATrueBoolean".to_owned(), Value::Boolean(true));
    dict.insert("AFalseBoolean".to_owned(), Value::Boolean(false));

    // Serialize dictionary as an XML plist.
    let mut buf = Cursor::new(Vec::new());
    crate::to_writer_xml(&mut buf, &dict).unwrap();
    let buf = buf.into_inner();
    let xml = std::str::from_utf8(&buf).unwrap();

    let comparison = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<d>
\t<k>ADictionary</k>
\t<d>
\t\t<k>FirstKey</k>
\t\t<s>FirstValue</s>
\t\t<k>SecondKey</k>
\t\t<r>1.234</r>
\t</d>
\t<k>AnInteger</k>
\t<i>123</i>
\t<k>ATrueBoolean</k>
\t<t/>
\t<k>AFalseBoolean</k>
\t<f/>
</d>
</plist>";

    assert_eq!(xml, comparison);
}

#[test]
fn serde_yaml_to_value() {
    let value: Value = serde_yaml::from_str("true").unwrap();
    assert_eq!(value, Value::Boolean(true));
}
