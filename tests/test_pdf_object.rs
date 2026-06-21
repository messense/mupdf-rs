use mupdf::pdf::{PdfDocument, PdfObject};
use mupdf::Error;

/// Open the shared test fixture for building array/dict objects bound to a document.
fn open() -> PdfDocument {
    PdfDocument::from_bytes(include_bytes!("files/dummy.pdf")).unwrap()
}

#[test]
fn array_iter_yields_pushed_values_in_order() -> Result<(), Error> {
    let doc = open();
    let mut arr = doc.new_array()?;
    arr.array_push(PdfObject::new_int(1)?)?;
    arr.array_push(PdfObject::new_int(2)?)?;
    arr.array_push(PdfObject::new_int(3)?)?;

    let collected: Vec<i32> = arr
        .array_iter()?
        .map(|item| item?.as_int())
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(collected, vec![1, 2, 3]);

    // ExactSizeIterator reports the remaining length.
    assert_eq!(arr.array_iter()?.len(), 3);
    Ok(())
}

#[test]
fn array_iter_empty() -> Result<(), Error> {
    let doc = open();
    let arr = doc.new_array()?;
    assert_eq!(arr.array_iter()?.count(), 0);
    assert_eq!(arr.array_iter()?.len(), 0);
    Ok(())
}

/// Soundness: the wrapper keeps a reference on every get, so iterated elements
/// must stay valid even after the source array is dropped.
#[test]
fn array_iter_values_outlive_source() -> Result<(), Error> {
    let doc = open();
    let values: Vec<PdfObject> = {
        let mut arr = doc.new_array()?;
        arr.array_push(PdfObject::new_int(1)?)?;
        arr.array_push(PdfObject::new_string("hi")?)?;
        arr.array_push(PdfObject::new_int(3)?)?;
        arr.array_iter()?.collect::<Result<Vec<_>, _>>()?
        // `arr` dropped here: releases only its own reference.
    };
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].as_int()?, 1);
    assert_eq!(values[1].as_string()?, "hi");
    assert_eq!(values[2].as_int()?, 3);
    Ok(())
}

/// Owned elements can themselves be iterated, forming borrow chains safely.
#[test]
fn array_iter_nested() -> Result<(), Error> {
    let doc = open();
    let mut inner_a = doc.new_array()?;
    inner_a.array_push(PdfObject::new_int(7)?)?;
    inner_a.array_push(PdfObject::new_int(8)?)?;
    let mut inner_b = doc.new_array()?;
    inner_b.array_push(PdfObject::new_int(9)?)?;

    let mut outer = doc.new_array()?;
    outer.array_push(inner_a)?;
    outer.array_push(inner_b)?;

    let collected: Vec<Vec<i32>> = outer
        .array_iter()?
        .map(|item| {
            let item = item?;
            item.array_iter()?
                .map(|i| i?.as_int())
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(collected, vec![vec![7, 8], vec![9]]);
    Ok(())
}

/// Iterating a non-array must fail loudly rather than silently yield nothing.
#[test]
fn array_iter_rejects_non_array() -> Result<(), Error> {
    let doc = open();
    let dict = doc.new_dict()?;
    assert!(matches!(dict.array_iter(), Err(Error::InvalidArgument(_))));
    Ok(())
}

#[test]
fn dict_iter_rejects_non_dict() -> Result<(), Error> {
    let doc = open();
    let arr = doc.new_array()?;
    assert!(matches!(arr.dict_iter(), Err(Error::InvalidArgument(_))));
    Ok(())
}

#[test]
fn dict_iter_yields_inserted_pairs() -> Result<(), Error> {
    let doc = open();
    let mut dict = doc.new_dict()?;
    dict.dict_put("Type", PdfObject::new_name("Catalog")?)?;
    dict.dict_put("Count", PdfObject::new_int(42)?)?;

    let mut count: Option<i32> = None;
    let mut type_name: Option<Vec<u8>> = None;
    for pair in dict.dict_iter()? {
        let (key, val) = pair?;
        let key = key.as_name()?;
        if &key[..] == b"Type" {
            type_name = Some(val.as_name()?);
        } else if &key[..] == b"Count" {
            count = Some(val.as_int()?);
        }
    }
    assert_eq!(count, Some(42));
    assert_eq!(type_name, Some(b"Catalog".to_vec()));
    assert_eq!(dict.dict_iter()?.len(), 2);
    Ok(())
}

#[test]
fn dict_iter_empty() -> Result<(), Error> {
    let doc = open();
    let dict = doc.new_dict()?;
    assert_eq!(dict.dict_iter()?.count(), 0);
    assert_eq!(dict.dict_iter()?.len(), 0);
    Ok(())
}

/// Soundness: keys and values stay valid after the source dict is dropped.
#[test]
fn dict_iter_pairs_outlive_source() -> Result<(), Error> {
    let doc = open();
    let pairs: Vec<(PdfObject, PdfObject)> = {
        let mut dict = doc.new_dict()?;
        dict.dict_put("A", PdfObject::new_int(1)?)?;
        dict.dict_put("B", PdfObject::new_int(2)?)?;
        dict.dict_iter()?.collect::<Result<Vec<_>, _>>()?
    };
    assert_eq!(pairs.len(), 2);
    let mut sum = 0;
    for (_, val) in pairs {
        sum += val.as_int()?;
    }
    assert_eq!(sum, 3);
    Ok(())
}
