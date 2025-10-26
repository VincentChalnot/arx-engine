const boardContainer = document.getElementById('board-container');
const statusDiv = document.getElementById('status');
const unstackModal = document.getElementById('unstack-modal');
const moveStackBtn = document.getElementById('move-stack');
const moveUnstackBtn = document.getElementById('move-unstack');
const switchSidesBtn = document.getElementById('switch-sides-btn');

let config = null;
let boardData = null;
let possibleMoves = [];
let selectedPiece = null; // { from: int, to: int[] }
let selectedMove = null; // { from: int, to: int }
let boardFlipped = false; // Track if the board is flipped
let boardCells = []; // Store references to board cells

const BOARD_SIZE = 9;
const LAST_BOARD_INDEX = (BOARD_SIZE * BOARD_SIZE) - 1;

const PIECE_CODE = {
    0b001: 'S',
    0b010: 'J',
    0b011: 'C',
    0b100: 'P',
    0b101: 'G',
    0b110: 'D',
    0b111: 'B',
};

/**
 * Creates the board HTML structure dynamically
 */
function createBoard() {
    const columns = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I'];
    const rows = boardFlipped ? [1, 2, 3, 4, 5, 6, 7, 8, 9] : [9, 8, 7, 6, 5, 4, 3, 2, 1];
    const displayColumns = boardFlipped ? [...columns].reverse() : columns;
    
    const table = document.createElement('table');
    table.className = 'board';
    table.id = 'arx-board';
    
    // Create thead
    const thead = document.createElement('thead');
    const headerRow = document.createElement('tr');
    headerRow.appendChild(document.createElement('th')); // Empty corner
    displayColumns.forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        headerRow.appendChild(th);
    });
    headerRow.appendChild(document.createElement('th')); // Empty corner
    thead.appendChild(headerRow);
    table.appendChild(thead);
    
    // Create tbody with cells
    const tbody = document.createElement('tbody');
    boardCells = []; // Reset cells array
    
    rows.forEach((rowNum, rowIndex) => {
        const tr = document.createElement('tr');
        
        // Row number on the left
        const leftHeader = document.createElement('th');
        leftHeader.textContent = rowNum;
        tr.appendChild(leftHeader);
        
        // Create 9 cells for this row
        for (let colIndex = 0; colIndex < 9; colIndex++) {
            const td = document.createElement('td');
            tr.appendChild(td);
            boardCells.push(td); // Store cell reference
        }
        
        // Row number on the right
        const rightHeader = document.createElement('th');
        rightHeader.textContent = rowNum;
        tr.appendChild(rightHeader);
        
        tbody.appendChild(tr);
    });
    
    table.appendChild(tbody);
    
    // Create tfoot
    const tfoot = document.createElement('tfoot');
    const footerRow = document.createElement('tr');
    footerRow.appendChild(document.createElement('th')); // Empty corner
    displayColumns.forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        footerRow.appendChild(th);
    });
    footerRow.appendChild(document.createElement('th')); // Empty corner
    tfoot.appendChild(footerRow);
    table.appendChild(tfoot);
    
    // Clear and append to container
    boardContainer.innerHTML = '';
    boardContainer.appendChild(table);
}

function decodePiece(piece) {
    if (piece === 0) return '';
    const color = (piece >> 6) & 0b1;
    const payload = piece & 0b00111111;

    if (payload === 0b111000) {
        return { top: 'K', bottom: null, color: color };
    }

    const topCode = (payload >> 3) & 0b111;
    const bottomCode = payload & 0b111;

    if (topCode === 0) { // Single piece
        if (PIECE_CODE[bottomCode]) {
            return { top: PIECE_CODE[bottomCode], bottom: null, color: color };
        }
    } else { // Stacked piece
        if (PIECE_CODE[topCode] && PIECE_CODE[bottomCode]) {
            return { top: PIECE_CODE[topCode], bottom: PIECE_CODE[bottomCode], color: color };
        }
    }
    return ''; // Invalid code
}

function renderBoard() {
    const turn = boardData[81] === 1 ? "White" : "Black";
    statusDiv.innerText = `${turn}'s turn to play.`;
    
    // Update each cell using the stored cell references
    for (let pos = 0; pos < 81; pos++) {
        // Map position based on board orientation
        const visualIndex = boardFlipped ? (LAST_BOARD_INDEX - pos) : pos;
        const cell = boardCells[visualIndex];
        if (!cell) continue;
        
        const pieceVal = boardData[pos];
        const piece = decodePiece(pieceVal);
        cell.innerText = '';
        cell.className = '';
        
        if (piece) {
            let text = piece.top;
            if (piece.bottom) {
                text += `+${piece.bottom}`;
            }
            cell.innerText = text;
            cell.classList.add(piece.color === 1 ? 'white-piece' : 'black-piece');
        }
        
        if (selectedPiece && selectedPiece.from === pos) {
            cell.classList.add('selected');
        }
        if (selectedPiece && selectedPiece.to.includes(pos)) {
            cell.classList.add('possible-move');
        }
    }
}

async function getPossibleMoves() {
    const response = await fetch(`${config.backendUrl}/moves`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: boardData,
    });
    const buffer = await response.arrayBuffer();
    const moves = new Uint16Array(buffer);
    possibleMoves = Array.from(moves);
}

function getMovesForPiece(pos) {
    const moves = [];
    for (const move of possibleMoves) {
        const from = move & 0x7F;
        const to = (move >> 7) & 0x7F;
        if (from === pos) {
            moves.push(to);
        }
    }
    return moves;
}

function isStacked(pos) {
    const pieceVal = boardData[pos];
    const payload = pieceVal & 0b0111111;
    const topCode = (payload >> 3) & 0b111;
    return topCode !== 0;
}

async function playMove(from, to, unstack = false) {
    // Find the move in possibleMoves
    let moveFound = null;
    for (const move of possibleMoves) {
        const moveFrom = move & 0x7F;
        const moveTo = (move >> 7) & 0x7F;
        const moveUnstack = (move >> 14) & 0x1;

        if (moveFrom === from && moveTo === to) {
            if (isStacked(from)) {
                if ((unstack && moveUnstack === 1) || (!unstack && moveUnstack === 0)) {
                    moveFound = move;
                    break;
                }
            } else {
                moveFound = move;
                break;
            }
        }
    }

    if (moveFound === null) {
        console.error("Move not found");
        return;
    }

    const moveBuffer = new Uint16Array([moveFound]).buffer;
    const payload = new Uint8Array(boardData.length + 2);
    payload.set(new Uint8Array(boardData), 0);
    payload.set(new Uint8Array(moveBuffer), boardData.length);

    const response = await fetch(`${config.backendUrl}/play`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: payload,
    });

    const newBoardBuffer = await response.arrayBuffer();
    boardData = new Uint8Array(newBoardBuffer);

    // Update URL
    window.location.hash = btoa(String.fromCharCode.apply(null, boardData));

    selectedPiece = null;
    selectedMove = null;
    await getPossibleMoves();
    renderBoard();
}

boardContainer.addEventListener('click', (e) => {
    const cell = e.target.closest('td');
    if (!cell) return;
    
    // Find the position by finding the cell index in our boardCells array
    let visualIndex = boardCells.indexOf(cell);
    if (visualIndex === -1) return;
    
    // Map visual index back to actual position based on orientation
    const pos = boardFlipped ? (LAST_BOARD_INDEX - visualIndex) : visualIndex;

    if (selectedPiece) {
        if (selectedPiece.to.includes(pos)) {
            // This is a move
            selectedMove = { from: selectedPiece.from, to: pos };
            if (isStacked(selectedPiece.from)) {
                // Show modal
                unstackModal.classList.add('is-active');
            } else {
                playMove(selectedMove.from, selectedMove.to, false);
            }
        } else {
            // Clicked somewhere else, deselect
            selectedPiece = null;
            renderBoard();
        }
    } else {
        const moves = getMovesForPiece(pos);
        if (moves.length > 0) {
            selectedPiece = { from: pos, to: moves };
            renderBoard();
        }
    }
});

moveStackBtn.addEventListener('click', () => {
    unstackModal.classList.remove('is-active');
    if (selectedMove) {
        playMove(selectedMove.from, selectedMove.to, false);
    }
});

moveUnstackBtn.addEventListener('click', () => {
    unstackModal.classList.remove('is-active');
    if (selectedMove) {
        playMove(selectedMove.from, selectedMove.to, true);
    }
});

// Close modal
document.querySelector('#unstack-modal .modal-background').addEventListener('click', () => {
    unstackModal.classList.remove('is-active');
    selectedPiece = null;
    selectedMove = null;
    renderBoard();
});

// Switch sides button handler
switchSidesBtn.addEventListener('click', () => {
    boardFlipped = !boardFlipped;
    selectedPiece = null;
    selectedMove = null;
    createBoard();
    renderBoard();
});


async function init() {
    // Show loading message
    statusDiv.innerText = 'Loading...';
    
    // Create the empty board structure first
    createBoard();
    
    // Then fetch config and initialize game
    const response = await fetch(`/config.json`);
    config = await response.json();

    if (window.location.hash) {
        try {
            const base64Board = window.location.hash.substring(1);
            const binaryString = atob(base64Board);
            const len = binaryString.length;
            const bytes = new Uint8Array(len);
            for (let i = 0; i < len; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            boardData = bytes;
        } catch (e) {
            console.error("Failed to load board from URL, starting new game.", e);
            const response = await fetch(`${config.backendUrl}/new`);
            const buffer = await response.arrayBuffer();
            boardData = new Uint8Array(buffer);
        }
    } else {
        const response = await fetch(`${config.backendUrl}/new`);
        const buffer = await response.arrayBuffer();
        boardData = new Uint8Array(buffer);
    }

    await getPossibleMoves(config);
    renderBoard();
}

init();
