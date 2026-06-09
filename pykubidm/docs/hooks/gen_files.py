import sys
from pathlib import Path

import mkdocs_gen_files


def files_match(file1: Path, file2: Path) -> bool:
    """Check if two files have identical content."""
    if not file1.exists() or not file2.exists():
        return False
    return file1.read_bytes() == file2.read_bytes()


def main() -> None:

    pykubidm_dir = Path(__file__).parent.parent.parent
    docs_dir = pykubidm_dir / "docs"
    readme_src = pykubidm_dir / "README.md"
    target_index = Path("index.md")
    if not readme_src.exists():
        raise FileNotFoundError(readme_src)

    readme_text = readme_src.read_text(encoding="utf-8")
    with mkdocs_gen_files.open(target_index, "w", encoding="utf-8") as fp:
        fp.write(readme_text)
        print(f"Generated README.md for documentation at {docs_dir / target_index}", file=sys.stderr)

    mkdocs_gen_files.set_edit_path(target_index, readme_src)

    workspace_dir = pykubidm_dir.parent
    logo_small = workspace_dir / "artwork/logo-small.png"
    target_logo = docs_dir / "assets/logo-small.png"

    if not logo_small.exists():
        print(f"Warning: logo-small.png not found at {logo_small}", file=sys.stderr)
        return

    if target_logo.exists() and files_match(logo_small, target_logo):
        print(f"logo-small.png already matches at {target_logo}, skipping generation", file=sys.stderr)
        return

    if not target_logo.parent.exists():
        target_logo.parent.mkdir(parents=True)
    
    with mkdocs_gen_files.open(target_logo, "wb") as fp:
        fp.write(logo_small.read_bytes())
        action = "Updated" if target_logo.exists() else "Generated"
        print(f"{action} logo-small.png for documentation at {target_logo}", file=sys.stderr)

    mkdocs_gen_files.set_edit_path(target_logo, logo_small)


main()
