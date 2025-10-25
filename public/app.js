const boardContainer = document.getElementById('board-container');
const boardTable = document.getElementById('arx-board');
const statusDiv = document.getElementById('status');
const unstackModal = document.getElementById('unstack-modal');
const moveStackBtn = document.getElementById('move-stack');
const moveUnstackBtn = document.getElementById('move-unstack');
const switchSidesBtn = document.getElementById('switch-sides');

let config = null;
let boardData = null;
let possibleMoves = [];
let selectedPiece = null; // { from: int, to: int[] }
let selectedMove = null; // { from: int, to: int }
let boardCells = null; // Cached board cells

// Get all board cells (td elements) in order
function getBoardCells() {
    if (!boardCells) {
        boardCells = Array.from(boardTable.querySelectorAll('tbody td'));
    }
    return boardCells;
}

// Get cell at a specific position
function getCellAtPosition(pos) {
    return getBoardCells()[pos] || null;
}

// Get position from a cell element
function getPositionFromCell(cell) {
    return getBoardCells().indexOf(cell);
}

const PIECE_CODE = {
    0b001: 'S',
    0b010: 'J',
    0b011: 'C',
    0b100: 'P',
    0b101: 'G',
    0b110: 'D',
    0b111: 'B',
};

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
    
    const cells = getBoardCells();
    
    // Update each cell
    for (let pos = 0; pos < 81; pos++) {
        const cell = cells[pos];
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

    const pos = getPositionFromCell(cell);
    if (pos === -1) return;

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

// Switch sides button
switchSidesBtn.addEventListener('click', () => {
    boardTable.classList.toggle('board-reversed');
});


async function init() {
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
