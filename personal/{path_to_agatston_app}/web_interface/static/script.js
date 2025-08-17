document.getElementById('scoreForm').addEventListener('submit', async function(event) {
    event.preventDefault();
    const age = document.getElementById('age').value;
    const gender = document.getElementById('gender').value;
    const calcium_score_lesion1 = document.getElementById('calcium_score_lesion1').value;

    // Placeholder for API call
    alert('Form submitted! Age: ' + age + ', Gender: ' + gender + ', Lesion 1: ' + calcium_score_lesion1);
    document.getElementById('agatstonScore').innerText = 'Agatston Score: Calculating...';
});