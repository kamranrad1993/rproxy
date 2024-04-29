
use os_info;

fn main() {
    let info = os_info::get();
            match info.os_type() {
                os_info::Type::Ubuntu => {
                    match info.version() {
                        os_info::Version::Semantic(major, minor, patch) => {
                            if *major == 22{
                                println!("cargo:rustc-cfg=feature=\"ubuntu-22\"");
                            }else if *major == 22 {
                                println!("cargo:rustc-cfg=feature=\"ubuntu-20\"");
                            }else {
                                panic!("Build failed due unsupported os");
                            }
                        },
                        os_info::Version::Rolling(_)|
                        os_info::Version::Custom(_)|
                        os_info::Version::Unknown => {
                            panic!("unknow os version {}-{} ", info.os_type(),info.version())
                        },
                    }
                }
                os_info::Type::AIX|
                os_info::Type::AlmaLinux|
                os_info::Type::Alpaquita|
                os_info::Type::Alpine|
                os_info::Type::Amazon|
                os_info::Type::Android|
                os_info::Type::Arch|
                os_info::Type::Artix|
                os_info::Type::CentOS|
                os_info::Type::Debian|
                os_info::Type::DragonFly|
                os_info::Type::Emscripten|
                os_info::Type::EndeavourOS|
                os_info::Type::Fedora|
                os_info::Type::FreeBSD|
                os_info::Type::Garuda|
                os_info::Type::Gentoo|
                os_info::Type::HardenedBSD|
                os_info::Type::Illumos|
                os_info::Type::Kali|
                os_info::Type::Linux|
                os_info::Type::Mabox|
                os_info::Type::Macos|
                os_info::Type::Manjaro|
                os_info::Type::Mariner|
                os_info::Type::MidnightBSD|
                os_info::Type::Mint|
                os_info::Type::NetBSD|
                os_info::Type::NixOS|
                os_info::Type::OpenBSD|
                os_info::Type::OpenCloudOS|
                os_info::Type::openEuler|
                os_info::Type::openSUSE|
                os_info::Type::OracleLinux|
                os_info::Type::Pop|
                os_info::Type::Raspbian|
                os_info::Type::Redhat|
                os_info::Type::RedHatEnterprise|
                os_info::Type::Redox|
                os_info::Type::RockyLinux|
                os_info::Type::Solus|
                os_info::Type::SUSE|
                os_info::Type::Ultramarine|
                os_info::Type::Void|
                os_info::Type::Unknown|
                os_info::Type::Windows|
                _ => {
                    panic!("Build failed due unsupported os {}-{} ", info.os_type(),info.version());
                }
            }

    
    // Example: Set a conditional compilation variable based on the current time
    // let current_time = chrono::Local::now();
    // let hour = current_time.hour();
    // if hour < 12 {
    //     println!("cargo:rustc-cfg=feature=\"morning\"");
    // } else {
    //     println!("cargo:rustc-cfg=feature=\"afternoon\"");
    // }
}