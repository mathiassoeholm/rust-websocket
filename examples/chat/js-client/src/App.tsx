import React, {useEffect} from 'react';

interface AppProps {}

function App({}: AppProps) {
  useEffect(() => {
    const socket = new WebSocket('ws://localhost:3000');
    socket.addEventListener('open', () => console.log('Opened!'));
    socket.addEventListener('message', console.log);
    socket.addEventListener('error', console.error);
    socket.addEventListener('close', console.log);
  }, []);

  return (
    <div>

    </div>
  )
}

export default App;
