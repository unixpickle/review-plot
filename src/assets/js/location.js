var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
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
        [this.latitude, this.longitude, this.accuracy].forEach((el) => {
            el.addEventListener('input', () => this.changedLocation = true);
        });
        this.fetchIPLocation();
        this.fetchBrowserLocation();
    }
    element() {
        return this._element;
    }
    urlEncode() {
        return (`latitude=${encodeURIComponent(this.latitude.value)}&` +
            `longitude=${encodeURIComponent(this.longitude.value)}&` +
            `accuracy=${encodeURIComponent(this.accuracy.value)}`);
    }
    fetchIPLocation() {
        return __awaiter(this, void 0, void 0, function* () {
            try {
                const result = yield (yield fetch('/api/location')).json();
                if (result == null) {
                    throw new Error('unknown location by IP');
                }
                console.log('location by IP:', result);
                if (this.changedLocation || this.gotBrowserLocation) {
                    return;
                }
                this.latitude.value = result[0].toString();
                this.longitude.value = result[1].toString();
            }
            catch (e) {
                console.log('failed to geolocate by IP: ' + e);
            }
        });
    }
    fetchBrowserLocation() {
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