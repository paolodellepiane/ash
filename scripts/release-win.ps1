$initialdir = pwd
cd $PSScriptRoot\..
$version = (cat ./Cargo.toml|Select-String -Pattern  '^version = \"(\d+\.\d+\.\d+)\"$').Matches.Groups[1].Value
if (Test-Path release) {
    rm -Recurse -Force release
}
mkdir release
cargo build --release
cp target\release\ash.exe release\
cp scripts\ash.json release\ash.json
cd release
Compress-Archive ash.exe ash-$version-win-x64.zip
(Get-FileHash .\ash-$version-win-x64.zip).Hash > .\ash-$version-win-x64.zip.sha256
gh release create v$version --generate-notes .\ash-$version-win-x64.zip .\ash-$version-win-x64.zip.sha256
& $env:UserProfile\scoop\apps\scoop\current\bin\checkver.ps1 ash . -Update
cat ash.json | gh gist edit 448ec9b86bcdf97faa1d7a3cd9d03d73 -f ash.json -
cd $initialdir