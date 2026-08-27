const controls = document.querySelector(".ui-lab-controls");
const preview = document.querySelector("#ui-lab-preview");

if (controls && preview) {
    const label = document.createElement("label");
    label.append("Guide");

    const select = document.createElement("select");
    select.id = "ui-lab-guide-mode";
    for (const [value, text] of [
        ["full", "Full"],
        ["subtle", "Subtle"],
        ["off", "Off"],
    ]) {
        const option = document.createElement("option");
        option.value = value;
        option.textContent = text;
        select.append(option);
    }

    label.append(select);
    controls.append(label);

    function apply() {
        preview.dataset.guideMode = select.value;
    }

    select.addEventListener("change", apply);
    apply();
}
