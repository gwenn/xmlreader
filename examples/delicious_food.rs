use std::fs;
use xmlreader::{Error, StreamReader, SubTreeReader};

struct Food {
    animals: Vec<Animal>,
    vegetables: Vec<Vegetable>,
}
struct Animal {
    name: String,
    meats: Vec<Meat>,
}
struct Meat {
    name: String,
}
struct Vegetable {
    name: String,
    preparations: Vec<String>,
}

/// Adapted from <a href="http://blog.palominolabs.com/2013/03/06/parsing-xml-with-java-and-staxmate/">Practical XML Parsing With Java and StaxMate</a>
/// (<a href="https://github.com/palominolabs/staxmate-example">StAX</a>)
fn main() -> Result<(), Error> {
    let xml = String::from_utf8(fs::read("sample.xml").unwrap()).unwrap();
    let mut sr = StreamReader::from(xml.as_str());
    let mut food = Food {
        animals: Vec::new(),
        vegetables: Vec::new(),
    };
    while sr.next_tag()?.is_some() {
        if sr.local_name()? == "animals" {
            animals(&mut sr, &mut food.animals)?;
        } else if sr.local_name()? == "vegetables" {
            vegetables(&mut sr, &mut food.vegetables)?;
        }
    }
    Ok(())
}

fn animals(sr: &mut StreamReader<'_>, animals: &mut Vec<Animal>) -> Result<(), Error> {
    let mut sr = SubTreeReader::new(sr)?;
    while sr.next_tag()?.is_some() {
        assert_eq!(sr.local_name()?, "animal");
        let mut animal = Animal {
            name: sr.attribute("name")?.unwrap().to_owned(),
            meats: Vec::new(),
        };
        meats(&mut sr, &mut animal.meats)?;
        animals.push(animal);
    }
    Ok(())
}
fn meats(sr: &mut SubTreeReader<'_, '_>, meats: &mut Vec<Meat>) -> Result<(), Error> {
    let mut sr = SubTreeReader::new(sr)?;
    while sr.next_tag()?.is_some() {
        if sr.local_name()? == "name" {
            meats.push(Meat {
                name: sr.element_text()?.unwrap().to_owned(),
            });
        }
    }
    Ok(())
}

fn vegetables(sr: &mut StreamReader<'_>, vegetables: &mut Vec<Vegetable>) -> Result<(), Error> {
    let mut sr = SubTreeReader::new(sr)?;
    while sr.next_tag()?.is_some() {
        assert_eq!(sr.local_name()?, "vegetable");
        let mut vegetable = Vegetable {
            name: "".to_owned(),
            preparations: Vec::new(),
        };
        crate::vegetable(&mut sr, &mut vegetable)?;
        vegetables.push(vegetable);
    }
    Ok(())
}
fn vegetable(sr: &mut SubTreeReader<'_, '_>, vegetable: &mut Vegetable) -> Result<(), Error> {
    let mut sr = SubTreeReader::new(sr)?;
    while sr.next_tag()?.is_some() {
        if sr.local_name()? == "name" {
            vegetable.name = sr.element_text()?.unwrap().to_owned();
        } else if sr.local_name()? == "preparations" {
            preparations(&mut sr, &mut vegetable.preparations)?;
        }
    }
    Ok(())
}
fn preparations(
    sr: &mut SubTreeReader<'_, '_>,
    preparations: &mut Vec<String>,
) -> Result<(), Error> {
    let mut sr = SubTreeReader::new(sr)?;
    while sr.next_tag()?.is_some() {
        if sr.local_name()? == "preparation" {
            preparations.push(sr.element_text()?.unwrap().to_owned());
        }
    }
    Ok(())
}
