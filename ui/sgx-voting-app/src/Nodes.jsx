import React from 'react';
import {Button, Card, Spinner, Table} from "react-bootstrap";
import * as Manager from './Manager';

class Nodes extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            nodes: null,
            pending: false
        };
    }

    refresh() {
        this.setState({pending: true});
        Manager.list_nodes().then((nodes) => this.setState({nodes, pending: false}));
    }

    componentDidMount() {
        this.refresh()
    }

    render() {
        const {nodes, pending} = this.state;
        if (nodes) {
            return <Card><Card.Body>
                <Table>
                    <caption>Free SGX Nodes</caption>
                    <thead>
                    <tr className="thead-dark">
                        <th>Name</th>
                        <th>NodeId</th>
                        <th>Subnet</th>
                    </tr>
                    </thead>
                    <tbody>
                    {nodes.map((node) => <tr>
                        <td>{node.name}</td>
                        <td>{node.nodeId}</td>
                        <td>{node.subnet}</td>
                    </tr>)}

                    </tbody>

                </Table></Card.Body>
                <Card.Footer><Button onClick={() => this.refresh()}>{pending ? <Spinner animation="border" size="sm"/> :
                    <i className="oi oi-reload"></i>} Refresh</Button></Card.Footer>
            </Card>;
        } else {
            return <Card><Card.Body><Spinner animation="grow"/></Card.Body></Card>;
        }
    }
}

export default Nodes;