# Chris Code

## Architecture

```mermaid
flowchart TB
    subgraph main["Main thread"]
        loop["Event loop\nrecv() on channel"]
        handler["handle_event()\n&mut AppState"]
        render["render()\nread AppState"]
        state["AppState\nmessages · streaming_response · mode"]

        loop -->|next event| handler
        handler -->|state updated| render
        handler -.->|mutates| state
        render -.->|reads| state
    end

    subgraph workers["Worker threads"]
        input["Input thread\ncrossterm events"]
        llm["LLM thread\ntokens · tool calls"]
    end

    channel["MPSC channel\nSender cloned per thread\nReceiver owned by main"]

    input -->|AppEvent::Input| channel
    llm -->|AppEvent::Llm| channel
    channel -->|AppEvent| loop
```
