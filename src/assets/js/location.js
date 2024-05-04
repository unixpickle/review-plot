class LocationPicker {
    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'location-picker';
        this.latitudeInput = createLabeledInput(this._element, 'Lat', '37.63');
        this.longitudeInput = createLabeledInput(this._element, 'Lon', '-122.44');
        this.accuracyInput = createLabeledInput(this._element, 'Acc', '10.0');
    }
    element() {
        return this._element;
    }
    urlQuery() {
        return (`latitude=${encodeURIComponent(this.latitudeInput.value)}&` +
            `longitude=${encodeURIComponent(this.longitudeInput.value)}&` +
            `accuracy=${encodeURIComponent(this.longitudeInput.value)}`);
    }
}
function createLabeledInput(parent, name, defaultVal) {
    const container = document.createElement('div');
    container.className = 'location-picker-field';
    const label = document.createElement('label');
    label.textContent = name;
    const content = document.createElement('input');
    content.value = defaultVal;
    container.appendChild(label);
    container.appendChild(content);
    parent.appendChild(container);
    return content;
}
//# sourceMappingURL=location.js.map