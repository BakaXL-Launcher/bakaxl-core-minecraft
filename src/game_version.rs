use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null;
use std::vec;
use std::{fs::File, env::consts::OS};
use std::io::Read;
use serde_json::Value;
use zip::read::ZipArchive;

use crate::json::json_version::{JsonAdvanceArgument, JsonLibrary};
use crate::tools::file_tools::lib_name_to_path;
use crate::tools::system_tools;
use crate::{json::{json_version::JsonVersion, self}, launcher_core::LauncherCore};

pub struct GameVersion {
    pub id: String,
    pub path: String,
    pub version_json: JsonVersion,
    launcher_core: LauncherCore
}

#[derive(Default)]
pub struct Libraries {
    pub libs: Vec<JsonLibrary>,
    pub natives: Vec<JsonLibrary>
}

impl GameVersion {
    /// 加载版本信息
    /// 
    /// # 参数
    /// `core`: LauncherCore，需要先new一个，例如```LauncherCore::new("路径")```
    /// 
    /// `version`: String，版本名字
    /// 
    /// 加载后将返回一个GameVersion类型，然后可以获取到版本的json文件信息
    pub fn load(core: LauncherCore, version: String) -> Self {
        let path = format!("{}/versions/{version}", core.base_path);
        let json_file = format!("{}/versions/{version}/{version}.json", core.base_path.clone());
        let mut file = File::open(json_file.clone()).expect("Failed to open file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read file");
        
        let minecraft_json: JsonVersion = serde_json::from_str(&contents).expect("Failed to parse JSON");
        Self { version_json: minecraft_json, launcher_core: core, path, id: version }
    }


    pub fn get_libraries_and_natives(self) -> Libraries {
        let mut libs: Vec<JsonLibrary> = vec![];
        let mut natives: Vec<JsonLibrary> = vec![];
        let libraries_json = self.version_json.libraries.unwrap();
        for lib in libraries_json {
            if lib.name.contains("natives") {
                natives.push(lib)
            } else {
                libs.push(lib)
            }
        }
        Libraries { libs, natives }
    }


    /// 获取未替换变量的启动参数，兼容以前版本的json参数`minecraftArguments`
    pub fn get_arguments(self) -> Vec<String> {
        let argument_old = self.version_json.arguments_old.unwrap_or_default();
        if !argument_old.is_empty() {
            vec![argument_old]
        } else {
            let mut argument_str: Vec<String> = vec![];
            for jvm_argument in self.version_json.arguments.clone().unwrap().jvm.unwrap() {
                match jvm_argument {
                    Value::String(s) => {
                        argument_str.push(s)
                    }
                    Value::Object(object) => {
                        let advance_argument: JsonAdvanceArgument = serde_json::from_value(serde_json::Value::Object(object)).unwrap();
                        for rule in advance_argument.rules.unwrap() {
                            if rule.action == "allow" && rule.os.is_some() {
                                if rule.os.clone().unwrap().name.is_some() {
                                    if OS.to_lowercase() != rule.os.clone().unwrap().name.unwrap_or_default().to_lowercase() {
                                        continue;
                                    } else if rule.os.clone().unwrap().version.is_some() /* 暂时还没想好怎么判断系统版本号 */ {
                                        continue;
                                    }
                                } 
                                if rule.os.clone().unwrap().arch.is_some() && rule.os.clone().unwrap().arch.unwrap() != system_tools::arch() {
                                    continue;
                                }

                                match &advance_argument.value {
                                    Value::String(s) => {
                                        argument_str.push(s.to_string())
                                    }
                                    Value::Array(arr) => {
                                        for arg in arr {
                                            argument_str.push(arg.to_string())
                                        }
                                    }
                                    _ => {
                                        
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        
                    }
                    
                }
            }

            for game_argument in self.version_json.arguments.clone().unwrap().game.unwrap() {
                match game_argument {
                    Value::String(s) => {
                        argument_str.push(s)
                    }
                    Value::Object(object) => {
                        let advance_argument: JsonAdvanceArgument = serde_json::from_value(serde_json::Value::Object(object)).unwrap();
                        for rule in advance_argument.rules.unwrap() {
                            if rule.action == "allow" {
                                match &advance_argument.value {
                                    Value::String(s) => {
                                        argument_str.push(s.to_string())
                                    }
                                    Value::Array(arr) => {
                                        for arg in arr {
                                            argument_str.push(arg.to_string())
                                        }
                                    }
                                    _ => { }
                                }
                            }
                        }
                    }
                    _ => {
                        
                    }
                    
                }
            }
            argument_str
        }
    }

    pub fn launch(self) -> String {
        let mut cp_str: String = String::default();
        let assets_path = self.launcher_core.assets_path.clone();
        let natives_path = format!("{}/natives-{}", self.path.clone(), OS);
        let libs_and_natives = self.get_libraries_and_natives();
        for lib in libs_and_natives.libs {
            if lib.clone().downloads.is_some() && lib.clone().downloads.unwrap().artifact.path.is_some() {
                cp_str += &format!("{assets_path}/{};", lib.clone().downloads.unwrap().artifact.path.unwrap());
            }
        }

        for native in libs_and_natives.natives {
            let file_path = format!("{}/libraries/{}", assets_path, lib_name_to_path(native.name));
            println!("{}", file_path);
            let file = File::open(file_path).expect("Failed open file");
            let mut archive = ZipArchive::new(file).expect("Failed open Zip file");
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).expect("Failed to get file from ZIP archive");
                let file_path = PathBuf::from(file.name());
                let output_path = Path::new(&natives_path).join(&file_path);
        
                if (&*file.name()).ends_with('/') {
                    // 创建目录
                    std::fs::create_dir_all(&output_path).expect("Failed to create directory");
                } else {
                    // 创建文件
                    if let Some(parent_dir) = output_path.parent() {
                        std::fs::create_dir_all(&parent_dir).expect("Failed to create parent directory");
                    }
        
                    let mut output_file = File::create(&output_path).expect("Failed to create file");
                    std::io::copy(&mut file, &mut output_file).expect("Failed to extract file");
                }
            }
        }

        cp_str

        
        /*
        let command = match OS {
            "windows" => {
                Command::new("cmd")
                    .args(&["/C", ""])
            }
        };
         */
    }
}