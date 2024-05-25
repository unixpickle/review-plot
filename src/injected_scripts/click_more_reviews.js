let buttons = Array.from(document.getElementsByTagName('button')).filter((x) => {
    const attr = x.getAttribute('jsaction');
    return attr && attr.endsWith('reviewChart.moreReviews');
});
if (buttons.length) {
    buttons[0].click();
    return true;
} else {
    return false;
}