param ($version)
cargo build --release
mkdir release
cp target\release\ash.exe release\
cp scripts\ash.json release\ash.json
cd release
Compress-Archive ash.exe ash-$version-win-x64.zip
(Get-FileHash .\ash-$version-win-x64.zip).Hash > .\ash-$version-win-x64.zip.sha256
gh release create v$version --generate-notes .\ash-$version-win-x64.zip .\ash-$version-win-x64.zip.sha256
& $env:UserProfile\scoop\apps\scoop\current\bin\checkver.ps1 ash . -Update
cat ash.json | gh gist edit 80e7005c4fd62cec9161f74bc2ad24ff -f ash.json -
cd ..