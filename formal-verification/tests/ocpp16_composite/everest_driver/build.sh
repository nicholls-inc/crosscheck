#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
driver_dir="$PWD"
mkdir -p .deps build
# Keep compiler scratch files inside the repository as well.
export TMPDIR="$driver_dir/build"
revision=dd696165d407d461a724232a4d77fe476a964ef1

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

# Every download is checked against the hash of the copy the DRT results were
# produced with, so a moved tag or a truncated file fails the build. GitHub
# does not promise that codeload archives stay byte-identical, so a mismatch on
# an archive may mean GitHub regenerated it rather than that its content changed.
download() {
    local url="$1" dest="$2" want="$3"
    if [[ ! -s "$dest" ]]; then
        curl --fail --location --retry 3 "$url" -o "$dest.part"
        mv "$dest.part" "$dest"
    fi
    local got
    got="$(sha256 "$dest")"
    if [[ "$got" != "$want" ]]; then
        rm -f "$dest"
        echo "FAIL: $dest had sha256 $got, expected $want. Removed it; re-run to download it again." >&2
        exit 1
    fi
}

download "https://codeload.github.com/EVerest/EVerest/tar.gz/$revision" .deps/everest.tar.gz \
    64a902ec5476bd1fd235439a96afdbc704d18e8805c138bd2ebdc2be1aab9431
if [[ ! -d ".deps/EVerest-$revision" ]]; then
    tar -xzf .deps/everest.tar.gz -C .deps
fi
download https://codeload.github.com/HowardHinnant/date/tar.gz/v3.0.1 .deps/date.tar.gz \
    7a390f200f0ccd207e8cff6757e04817c1a0aec3e327b006b7eb451c57ee3538
if [[ ! -d .deps/date-3.0.1 ]]; then
    tar -xzf .deps/date.tar.gz -C .deps
fi
mkdir -p .deps/include/nlohmann
download https://raw.githubusercontent.com/nlohmann/json/v3.11.3/single_include/nlohmann/json.hpp .deps/include/nlohmann/json.hpp \
    9bea4c8066ef4a1c206b2be5a36302f8926f7fdc6087af5d20b417d0cf103ea6
download https://raw.githubusercontent.com/nlohmann/json/v3.11.3/single_include/nlohmann/json_fwd.hpp .deps/include/nlohmann/json_fwd.hpp \
    5cefbba751baf5243033fd894c70be9b37103d64b3d3a959e4173197d71137b9

ocpp="$driver_dir/.deps/EVerest-$revision/lib/everest/ocpp"
date_dir="$driver_dir/.deps/date-3.0.1"
compiler="${CXX:-c++}"
includes=(-I"$driver_dir/shim" -I"$ocpp/include" -I"$driver_dir/.deps/include"
          -I"$date_dir/include" -I/opt/homebrew/include)
flags=(-std=c++17 -O2 -DUSE_OS_TZDB=1 -DHAS_REMOTE_API=0 -DFMT_HEADER_ONLY
       -ffunction-sections -fdata-sections -pthread)
if [[ "$(uname -s)" == Darwin ]]; then
    link_flags=(-Wl,-dead_strip)
else
    link_flags=(-Wl,--gc-sections)
fi
# Compile complete upstream translation units; discard unused code at link time.
sources=("$driver_dir/main.cpp" "$ocpp/lib/ocpp/v16/profile.cpp"
         "$ocpp/lib/ocpp/v16/ocpp_types.cpp" "$ocpp/lib/ocpp/v16/ocpp_enums.cpp"
         "$ocpp/lib/ocpp/common/types.cpp" "$date_dir/src/tz.cpp")
objects=()
for index in "${!sources[@]}"; do
    object="$driver_dir/build/$index.o"
    "$compiler" "${flags[@]}" "${includes[@]}" -c "${sources[$index]}" -o "$object"
    objects+=("$object")
done
"$compiler" -pthread "${link_flags[@]}" "${objects[@]}" -o build/everest_composite
echo "Built $driver_dir/build/everest_composite"
