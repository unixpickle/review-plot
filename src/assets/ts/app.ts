interface Window {
    app: App;
}

class App {
    private searchBox: HTMLInputElement;
    private searchButton: HTMLButtonElement;

    constructor() {
        const searchContainer = document.createElement('div');
        searchContainer.className = 'search-container';
        this.searchBox = document.createElement('input');
        this.searchBox.placeholder = 'Name of business';
        this.searchButton = document.createElement('button');
        this.searchButton.textContent = 'Search';
        searchContainer.appendChild(this.searchBox);
        searchContainer.appendChild(this.searchButton);
        document.body.appendChild(searchContainer);
    }
}

window.app = new App();