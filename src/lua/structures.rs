use serde::{Serialize, Deserialize};
use rlua::{UserData, UserDataMethods, ToLua, Context, Value, Error, Table, FromLuaMulti, ToLuaMulti};

#[derive(Serialize, Deserialize)]
pub struct GameObject {
    pub id: i32,
    pub name: String,
}

// impl<'lua> ToLua<'lua> for GameObject {
//     fn to_lua(&self, lua: Context<'lua>) -> Result<Value<'lua>, Error> {
//         let table = lua.create_table().unwrap();
//         table.set("id", self.id);
// //        table.set("name", self.name);
// //        table.set("haha", haha);
//         Ok(Value::Table(table))
//     }
// }
//
// impl UserData for GameObject {
//     fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(_methods: &mut T) {
//         _methods.add_method()
//     }
// }


