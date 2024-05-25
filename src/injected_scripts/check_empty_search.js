const divs = document.getElementsByTagName('div');
for (let i = 0; i < divs.length; i++) {
    if (divs[i].textContent.startsWith("Google Maps can't find")) {
        return true;
    }
}
return false;