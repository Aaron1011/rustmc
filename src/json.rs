use rustc_serialize::json;
use rustc_serialize::json::Json;

pub struct ExtraJSON(json::Json);

impl ExtraJSON {
    pub fn new(j: json::Json) -> ExtraJSON {
        ExtraJSON(j)
    }

    pub fn string(&self) -> String {
        match *self {
            ExtraJSON(Json::String(ref s)) => s.clone(),
            _ => panic!("tried to get string from non-string")
        }
    }

    pub fn list(&self) -> Vec<ExtraJSON> {
        self.list_map(|x| x)
    }

    pub fn list_map<T, F>(&self, f: F) -> Vec<T> where F: Fn(ExtraJSON) -> T {
        match *self {
            ExtraJSON(Json::Array(ref l)) => {
                l.iter().map(|x| f(ExtraJSON(x.clone()))).collect()
            }
            _ => panic!("tried to get list from non-list")
        }
    }

    pub fn as_int(&self) -> i64 {
        match *self {
            ExtraJSON(Json::I64(f)) => f as i64,
            _ => panic!("tried to convert non-int to int!")
        }
    }
}


/*trait ExtraJSONIndex {
    fn index(&self, j: &ExtraJSON) -> ExtraJSON;
}*/

/*impl Index<String, ExtraJSON> for ExtraJSON {
    fn index(&self, j: String) -> ExtraJSON {
        match *j {
            ExtraJSON(json::Object(ref ij)) => {
                match ij.find(&self.to_owned()) {
                    Some(jj) => ExtraJSON(jj.clone()),
                    None => panic!("no such key")
                }
            }
            _ => panic!("tried to index non-object with string")
        }
    }
}*/
/*
impl Index<int, ExtraJSON> for int {
    fn index(&self, j: &ExtraJSON) -> ExtraJSON {
        match *j {
            ExtraJSON(json::List(ref l)) => ExtraJSON(l[*self as uint].clone()),
            _ => panic!("tried to index non-list with int")
        }
    }
}*/

/*impl<'a> Index<&'a str, ExtraJSON> for ExtraJSON {
    fn index(&self, idx: &'a str) -> ExtraJSON {
        //idx.index(self)
    }
}*/
