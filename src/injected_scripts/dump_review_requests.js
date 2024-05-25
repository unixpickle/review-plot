const origOpen = XMLHttpRequest.prototype.open;
XMLHttpRequest.prototype.open = function (method, url) {
    this._url = url;
    return origOpen.apply(this, arguments);
};
const origSend = XMLHttpRequest.prototype.send;
window.recordedReviewResponses = [];
XMLHttpRequest.prototype.send = function () {
    const oldCb = this.onreadystatechange;
    this.onreadystatechange = function () {
        if (this.readyState == 4 && this._url.includes('listugcposts')) {
            let url = this._url;
            if (url.startsWith('/')) {
                url = location.origin + url;
            }
            window.recordedReviewResponses.push([url, this.response]);
        }
        if (oldCb) {
            return oldCb.apply(this, arguments);
        }
    };
    origSend.apply(this, arguments);
};