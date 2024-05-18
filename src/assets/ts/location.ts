class LocationPicker {
    private _element: HTMLElement;
    private collapseButton: HTMLButtonElement;
    private latitude: HTMLInputElement;
    private longitude: HTMLInputElement;
    private accuracy: HTMLInputElement;

    private gotBrowserLocation: boolean;
    private changedLocation: boolean;

    constructor() {
        this._element = document.createElement('div');
        this._element.className = 'location-picker location-picker-closed';

        const header = document.createElement('div');
        header.className = 'location-picker-header';
        this.collapseButton = document.createElement('button');
        this.collapseButton.addEventListener('click', () => {
            this._element.classList.toggle('location-picker-closed');
        })
        header.appendChild(this.collapseButton);
        const label = document.createElement('label');
        label.textContent = 'Search from geolocation';
        label.addEventListener('click', () => {
            this._element.classList.toggle('location-picker-closed');
        })
        header.appendChild(label);
        this._element.appendChild(header);

        this.latitude = createLabeledInput(this._element, 'Lat', '37.63');
        this.longitude = createLabeledInput(this._element, 'Lon', '-122.44');
        this.accuracy = createLabeledInput(this._element, 'Acc', '10.0');

        [this.latitude, this.longitude, this.accuracy].forEach((el) => {
            el.addEventListener('input', () => this.changedLocation = true);
        });

        this.fetchIPLocation();
        this.fetchBrowserLocation();
    }

    element(): HTMLElement {
        return this._element;
    }

    urlEncode(): string {
        return (`latitude=${encodeURIComponent(this.latitude.value)}&` +
            `longitude=${encodeURIComponent(this.longitude.value)}&` +
            `accuracy=${encodeURIComponent(this.accuracy.value)}`);
    }

    private async fetchIPLocation() {
        try {
            const result = await (await fetch('/api/location')).json();
            if (result == null) {
                throw new Error('unknown location by IP');
            }
            console.log('location by IP:', result);
            if (this.changedLocation || this.gotBrowserLocation) {
                return;
            }
            this.latitude.value = result[0].toString();
            this.longitude.value = result[1].toString();
        } catch (e) {
            console.log('failed to geolocate by IP: ' + e);
        }
    }

    private fetchBrowserLocation() {
        navigator.geolocation.getCurrentPosition((position) => {
            if (this.changedLocation) {
                return;
            }
            this.gotBrowserLocation = true;
            this.latitude.value = position.coords.latitude.toString();
            this.longitude.value = position.coords.longitude.toString();
            this.accuracy.value = position.coords.accuracy.toString();
        });
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