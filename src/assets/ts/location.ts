class LocationPicker {
    private _element: HTMLElement;
    private latitude: HTMLInputElement;
    private longitude: HTMLInputElement;
    private accuracy: HTMLInputElement;

    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'location-picker';
        this.latitude = createLabeledInput(this._element, 'Lat', '37.63');
        this.longitude = createLabeledInput(this._element, 'Lon', '-122.44');
        this.accuracy = createLabeledInput(this._element, 'Acc', '10.0');
    }

    element(): HTMLElement {
        return this._element;
    }

    urlEncode(): string {
        return (`latitude=${encodeURIComponent(this.latitude.value)}&` +
            `longitude=${encodeURIComponent(this.longitude.value)}&` +
            `accuracy=${encodeURIComponent(this.accuracy.value)}`);
    }
}

function createLabeledInput(parent: HTMLElement, name: string, defaultVal: string): HTMLInputElement {
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