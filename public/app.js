const boardContainer = document.getElementById('board-container');
const statusDiv = document.getElementById('status');
const unstackModal = document.getElementById('unstack-modal');
const moveStackBtn = document.getElementById('move-stack');
const moveUnstackBtn = document.getElementById('move-unstack');
const switchSidesBtn = document.getElementById('switch-sides-btn');
const moveHistoryTextarea = document.getElementById('move-history');
const loadGameBtn = document.getElementById('load-game-btn');
const undoBtn = document.getElementById('undo-btn');
const askEngineBtn = document.getElementById('ask-engine-btn');

let config = null;
let boardData = null;
let possibleMoves = [];
let selectedPiece = null; // { from: int, to: int[] }
let selectedMove = null; // { from: int, to: int }
let boardFlipped = false; // Track if the board is flipped
let boardCells = []; // Store references to board cells
let hoveredPiece = null; // Track currently hovered piece position
let moveHistory = []; // Array of moves in format "A1-B2"
let gameHistory = []; // Array of board states (Uint8Array)

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
        // Highlight hovered possible moves
        if (hoveredPiece !== null) {
            const hoveredMoves = getMovesForPiece(hoveredPiece);
            if (hoveredMoves.includes(pos) && (!selectedPiece || selectedPiece.from !== hoveredPiece)) {
                cell.classList.add('hovered-move');
            }
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

/**
 * Convert position index (0-80) to algebraic notation (A1-I9)
 */
function posToAlgebraic(pos) {
    const x = pos % 9;
    const y = Math.floor(pos / 9);
    const col = String.fromCharCode('A'.charCodeAt(0) + x);
    const row = 9 - y;
    return col + row;
}

/**
 * Convert algebraic notation (A1-I9) to position index (0-80)
 */
function algebraicToPos(algebraic) {
    if (!algebraic || algebraic.length < 2) return null;
    const col = algebraic[0].toUpperCase();
    const row = parseInt(algebraic.substring(1));
    if (col < 'A' || col > 'I' || row < 1 || row > 9) return null;
    const x = col.charCodeAt(0) - 'A'.charCodeAt(0);
    const y = 9 - row;
    return y * 9 + x;
}

/**
 * Update the move history textarea
 */
function updateMoveHistoryDisplay() {
    let text = '';
    for (let i = 0; i < moveHistory.length; i += 2) {
        text += moveHistory[i];
        if (i + 1 < moveHistory.length) {
            text += ' ' + moveHistory[i + 1];
        }
        text += '\n';
    }
    moveHistoryTextarea.value = text;
}

async function playMove(from, to, unstack = false) {
    let moveBits = (from & 0x7F) | ((to & 0x7F) << 7);
    if (unstack) {
        moveBits |= (1 << 14);
    }
    const moveBuffer = new Uint16Array([moveBits]).buffer;
    const payload = new Uint8Array(boardData.length + 2);
    payload.set(new Uint8Array(boardData), 0);
    payload.set(new Uint8Array(moveBuffer), boardData.length);

    const response = await fetch(`${config.backendUrl}/play`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: payload,
    });

    const newBoardBuffer = await response.arrayBuffer();

    // Save current board state to history before updating
    gameHistory.push(new Uint8Array(boardData));

    boardData = new Uint8Array(newBoardBuffer);

    // Update URL
    window.location.hash = btoa(String.fromCharCode.apply(null, boardData));

    // Record move in algebraic notation
    const moveNotation = posToAlgebraic(from) + '-' + posToAlgebraic(to);
    moveHistory.push(moveNotation);
    updateMoveHistoryDisplay();

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
    setTimeout(addBoardCellHoverListeners, 0);
});

// Ask Engine button handler
askEngineBtn.addEventListener('click', async () => {
    try {
        // Disable button while processing
        askEngineBtn.disabled = true;
        askEngineBtn.innerText = 'Thinking...';

        // Request engine move
        const response = await fetch(`${config.backendUrl}/engine-move`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/octet-stream' },
            body: boardData,
        });

        if (!response.ok) {
            throw new Error(`Server returned ${response.status}`);
        }

        const moveBuffer = await response.arrayBuffer();
        const moveArray = new Uint16Array(moveBuffer);
        const engineMove = moveArray[0];

        // Decode the move
        const from = engineMove & 0x7F;
        const to = (engineMove >> 7) & 0x7F;
        const unstack = (engineMove >> 14) & 0x1;

        // Apply the move
        await playMove(from, to, unstack === 1);

    } catch (error) {
        console.error('Error getting engine move:', error);
        statusDiv.innerText = `Error: ${error.message}. Engine may not be available.`;
    } finally {
        // Re-enable button
        askEngineBtn.disabled = false;
        askEngineBtn.innerText = 'Ask Engine';
    }
});

function addBoardCellHoverListeners() {
    boardCells.forEach((cell, visualIndex) => {
        cell.onmouseenter = () => {
            // Map visual index back to actual position
            const pos = boardFlipped ? (LAST_BOARD_INDEX - visualIndex) : visualIndex;
            const pieceVal = boardData[pos];
            const piece = decodePiece(pieceVal);
            // Only highlight if friendly piece and not currently selected
            if (piece && piece.color === boardData[81] && (!selectedPiece || selectedPiece.from !== pos)) {
                hoveredPiece = pos;
                renderBoard();
            }
        };
        cell.onmouseleave = () => {
            if (hoveredPiece !== null) {
                hoveredPiece = null;
                renderBoard();
            }
        };
    });
}

// Undo button handler
undoBtn.addEventListener('click', async () => {
    if (gameHistory.length === 0) {
        alert('No moves to undo');
        return;
    }

    // Restore previous board state
    boardData = gameHistory.pop();

    // Remove last move from history
    moveHistory.pop();
    updateMoveHistoryDisplay();

    // Update URL
    window.location.hash = btoa(String.fromCharCode.apply(null, boardData));

    selectedPiece = null;
    selectedMove = null;
    await getPossibleMoves();
    renderBoard();
});

// Load game button handler
loadGameBtn.addEventListener('click', async () => {
    const text = moveHistoryTextarea.value.trim();
    if (!text) {
        alert('Please enter moves to load');
        return;
    }

    // Parse moves from textarea
    const lines = text.split('\n');
    const moves = [];
    for (const line of lines) {
        const parts = line.trim().split(/\s+/);
        for (const part of parts) {
            if (part.includes('-')) {
                moves.push(part);
            }
        }
    }

    if (moves.length === 0) {
        alert('No valid moves found');
        return;
    }

    // Start a new game
    const response = await fetch(`${config.backendUrl}/new`);
    const buffer = await response.arrayBuffer();
    boardData = new Uint8Array(buffer);
    moveHistory = [];
    gameHistory = [];

    // Apply each move
    for (const moveNotation of moves) {
        const parts = moveNotation.split('-');
        if (parts.length !== 2) {
            alert(`Invalid move format: ${moveNotation}`);
            return;
        }

        const fromPos = algebraicToPos(parts[0]);
        const toPos = algebraicToPos(parts[1]);

        if (fromPos === null || toPos === null) {
            alert(`Invalid position in move: ${moveNotation}`);
            return;
        }

        // Get possible moves for current board state
        await getPossibleMoves();

        // Check if this move is legal
        const moves = getMovesForPiece(fromPos);
        if (!moves.includes(toPos)) {
            alert(`Illegal move: ${moveNotation}`);
            return;
        }

        // Always move full stack when loading from history
        await playMove(fromPos, toPos, false);
    }

    renderBoard();
});

async function init() {
    // Show loading message
    statusDiv.innerText = 'Loading...';
    
    // Create the empty board structure first
    createBoard();
    setTimeout(addBoardCellHoverListeners, 0);

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
            // When loading from URL, clear history since we don't know the moves
            moveHistory = [];
            gameHistory = [];
        } catch (e) {
            console.error("Failed to load board from URL, starting new game.", e);
            const response = await fetch(`${config.backendUrl}/new`);
            const buffer = await response.arrayBuffer();
            boardData = new Uint8Array(buffer);
            moveHistory = [];
            gameHistory = [];
        }
    } else {
        const response = await fetch(`${config.backendUrl}/new`);
        const buffer = await response.arrayBuffer();
        boardData = new Uint8Array(buffer);
        moveHistory = [];
        gameHistory = [];
    }

    await getPossibleMoves(config);
    renderBoard();
    updateMoveHistoryDisplay();
}

init();
