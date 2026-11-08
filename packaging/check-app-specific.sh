#!/usr/bin/env bash
# App-specific installer and package assertions retained alongside shared packaging lint.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

for want in '<UIRef Id="EvoLightUI" />' 'Value="InstallDirDlg"' '<Property Id="WIXUI_INSTALLDIR" Value="INSTALLFOLDER" />' \
    'Key="Software\EvoLight" Name="InstallFolder"' 'Value="LaunchEvoLight"'; do
  grep -qF "$want" packaging/windows/evolight.wxs || { echo "::error::evolight.wxs lost: $want" >&2; exit 1; }
done
grep -qF '<DialogRef Id="ExitDialog" />' packaging/windows/evolight.wxs \
  || { echo "::error::ExitDialog is not referenced; installs and removes end without a completion dialog" >&2; exit 1; }

python3 - <<'PYEOF'
import yaml
manifests = []
for name in ("ai.storyteller.evolight.yml", "ai.storyteller.evolight.bundle.yml"):
    with open("packaging/linux/flatpak/" + name) as source:
        manifest = yaml.safe_load(source)
    assert manifest["id"] == "ai.storyteller.evolight" and manifest["command"] == "evolight", name
    # Without this talk-name the sandbox hides the accessibility bus and Orca cannot see the app.
    assert "--talk-name=org.a11y.Bus" in manifest["finish-args"], f"{name} lost --talk-name=org.a11y.Bus"
    manifests.append(manifest)
for key in ("runtime", "runtime-version", "sdk", "finish-args"):
    assert manifests[0][key] == manifests[1][key], f"Flatpak manifests disagree on {key}"
print("EvoLight package identity ok")
PYEOF

# The macOS bundle must declare only file types EvoLight can open. It used to carry EvoPhoto's
# `.evophoto` document type and imported PSB/QOI declarations, so Launch Services typed EvoPhoto
# documents as EvoLight's and `.evophoto` had three claimant apps (issue #1951). plistlib parses the
# template as a property list too, which is the syntax check package.sh's `plutil -lint` repeats on
# the substituted file.
python3 - <<'PYEOF'
import plistlib
with open("packaging/macos/Info.plist.in", "rb") as source:
    plist = plistlib.load(source)
for key, value in {
    "CFBundleIdentifier": "ai.storyteller.evolight",
    "CFBundleExecutable": "EvoLight",
    "CFBundleIconFile": "EvoLight",
    "LSMinimumSystemVersion": "11.0",
}.items():
    assert plist.get(key) == value, f"Info.plist {key}: {plist.get(key)!r}, expected {value!r}"
for key in ("CFBundleShortVersionString", "CFBundleVersion", "EvoLightVersion", "EvoLightBuildCommit"):
    assert plist.get(key), f"Info.plist lost {key}"
declared = set()
for section in ("UTExportedTypeDeclarations", "UTImportedTypeDeclarations"):
    for entry in plist.get(section, []):
        tags = entry.get("UTTypeTagSpecification", {})
        declared.update(tags.get("public.filename-extension", []))
        declared.update(tags.get("public.mime-type", []))
        declared.add(entry.get("UTTypeIdentifier", ""))
unsupported = {name.lower() for name in declared} & {
    "evophoto",
    "psb",
    "qoi",
    "application/x-evolight",
    "ai.storyteller.evolight.document",
    "ai.storyteller.evolight.qoi",
    "com.adobe.photoshop-large-image",
}
assert not unsupported, f"Info.plist declares types EvoLight cannot open: {sorted(unsupported)}"
print("EvoLight macOS bundle types ok")
PYEOF
