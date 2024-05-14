class LocationPicker {
    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'location-picker location-picker-closed';
        const header = document.createElement('div');
        header.className = 'location-picker-header';
        this.collapseButton = document.createElement('button');
        this.collapseButton.addEventListener('click', () => {
            this._element.classList.toggle('location-picker-closed');
        });
        header.appendChild(this.collapseButton);
        const label = document.createElement('label');
        label.textContent = 'Search from geolocation';
        label.addEventListener('click', () => {
            this._element.classList.toggle('location-picker-closed');
        });
        header.appendChild(label);
        this._element.appendChild(header);
        this.latitude = createLabeledInput(this._element, 'Lat', '37.63');
        this.longitude = createLabeledInput(this._element, 'Lon', '-122.44');
        this.accuracy = createLabeledInput(this._element, 'Acc', '10.0');
    }
    element() {
        return this._element;
    }
    urlEncode() {
        return (`latitude=${encodeURIComponent(this.latitude.value)}&` +
            `longitude=${encodeURIComponent(this.longitude.value)}&` +
            `accuracy=${encodeURIComponent(this.accuracy.value)}`);
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