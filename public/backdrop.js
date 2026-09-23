document.querySelectorAll('button').forEach(button => button.addEventListener('click', () => { document.body.className = button.id === 'multi' ? '' : button.id; }));
