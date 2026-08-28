fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = env!("CARGO_PKG_VERSION");
    let parts: Vec<u16> = version
        .split('.')
        .filter_map(|s| s.parse::<u16>().ok())
        .collect();
    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let build = parts.get(2).copied().unwrap_or(0);
    let revision = parts.get(3).copied().unwrap_or(0);
    // FILEVERSION/PRODUCTVERSION 需要打包为 64 位: major.minor.build.revision
    let packed = (u64::from(major) << 48)
        | (u64::from(minor) << 32)
        | (u64::from(build) << 16)
        | u64::from(revision);

    // FileVersion 为 4 段（缺省补 0），ProductVersion 为版本号本身
    let file_version = format!("{}.{}.{}.{}", major, minor, build, revision);

    // 当前年份（本地时区，与 Copyright 的动态年份保持一致）
    let year = chrono::Local::now().format("%Y");

    let mut res = winresource::WindowsResource::new();
    // 语言设为英文（0x0409 = English US），避免版本资源显示"语言中立"
    res.set_language(0x0409);
    // 嵌入仓库根目录的 UAC 清单（requireAdministrator），根文件为唯一来源
    println!("cargo:rerun-if-changed=../app.manifest");
    let manifest = std::fs::read_to_string("../app.manifest").expect("app.manifest missing");
    res.set_manifest(&manifest);
    res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
    res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
    res.set("FileVersion", &file_version);
    res.set("ProductVersion", version);
    res.set("CompanyName", "NXRKYMANE SOFTWARE");
    res.set("ProductName", "Scandium");
    res.set("FileDescription", "scandium_svc");
    res.set(
        "LegalCopyright",
        &format!("Copyright (C) {} NXRKYMANE SOFTWARE", year),
    );
    res.set_icon("../Misc/Proj.ico");
    res.compile().unwrap();
}
